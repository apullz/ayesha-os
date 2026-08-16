import glob
import os
import socket as _socket
import subprocess
import threading
import time

import numpy as np

TERMUX_PREFIX = '/data/data/com.termux/files/usr'
TCP_BRIDGE = 'tcp:127.0.0.1:4713'


def termux_pacat():
    p = os.path.join(TERMUX_PREFIX, 'bin/pacat')
    return p if os.path.exists(p) else 'pacat'


def hivepipe():
    """bundled pulse-simple player (compiled with termux clang)."""
    p = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'hivepipe')
    return p if os.path.exists(p) else None


def _find_pulse_socket():
    candidates = [
        os.path.join(TERMUX_PREFIX, 'var/run/pulse/native'),
        os.path.join(TERMUX_PREFIX, 'tmp/pulse-*/native'),
    ]
    for pat in candidates:
        for m in sorted(glob.glob(pat)):
            if os.path.exists(m):
                return m
    return None


def _tcp_bridge_ok():
    """the proot↔pulse boundary breaks on unix-socket SCM_CREDENTIALS, so
    we prefer the loopback TCP bridge (auth-anonymous, loaded by the
    pulseaudio service / setup_termux.sh)."""
    try:
        s = _socket.create_connection(('127.0.0.1', 4713), timeout=0.4)
        s.close()
        return True
    except OSError:
        return False


def _to_stereo_bytes(mono):
    data = np.clip(np.repeat(mono, 2), -1.0, 1.0)
    return (data * 32767.0).astype(np.int16).tobytes()


def ssh_hivepipe():
    """run hivepipe on the real termux side over ssh: proot↔pulse sockets are
    dead (SCM_CREDENTIALS + broken TCP module), but a termux-side client works.
    env: HIVEBEAT_TERMUX_HOST / _PORT / _KEY"""
    host = os.environ.get('HIVEBEAT_TERMUX_HOST', 'u0_a248@127.0.0.1')
    port = os.environ.get('HIVEBEAT_TERMUX_PORT', '8022')
    key = os.environ.get('HIVEBEAT_TERMUX_KEY',
                         '/data/data/com.termux/files/home/.ssh/id_ed25519')
    return ['ssh', '-p', port, '-o', 'BatchMode=yes', '-o', 'ConnectTimeout=5',
            '-o', 'IdentitiesOnly=yes', '-i', key, host,
            '/data/data/com.termux/files/usr/bin/hivepipe']


class PacatSink:
    """live audio: stream s16le into hivepipe (bundled pulse-simple player),
    falling back to pacat. both talk to the termux pulseaudio daemon."""

    def __init__(self, engine, block=512):
        self.engine = engine
        self.block = block
        self.proc = None
        self.thread = None
        self.running = False

    def start(self):
        # preferred: hivepipe running on the real termux side via ssh — the
        # only path that survives proot (see ssh_hivepipe docstring).
        ssh_cmd = ssh_hivepipe()
        ssh_proc = self._probe(ssh_cmd, {})
        if ssh_proc:
            self.proc = ssh_proc
            self.running = True
            self.label = 'hivepipe (termux via ssh)'
            self.thread = threading.Thread(target=self._loop, daemon=True)
            self.thread.start()
            return
        hp = hivepipe()
        if hp:
            cmd = [hp]
            env = os.environ.copy()
            env['HIVEPIPE_RATE'] = str(self.engine.sr)
            env['HIVEPIPE_CH'] = '2'
            label = 'hivepipe'
        else:
            cmd = [termux_pacat(), '--playback', '--raw', '--format=s16le',
                   f'--rate={self.engine.sr}', '--channels=2', '--latency-msec=120']
            env = os.environ.copy()
            label = 'pacat'
        # prefer the TCP bridge (works from real termux); fall back to the
        # unix socket for plain termux setups.
        if _tcp_bridge_ok():
            env['PULSE_SERVER'] = TCP_BRIDGE
        else:
            sock = _find_pulse_socket()
            if sock:
                env['PULSE_SERVER'] = sock
        try:
            self.proc = subprocess.Popen(cmd, stdin=subprocess.PIPE,
                                         stdout=subprocess.DEVNULL,
                                         stderr=subprocess.DEVNULL, env=env)
        except FileNotFoundError:
            raise RuntimeError(f'{label} not found — run `pkg install pulseaudio` in termux')
        time.sleep(0.6)
        if self.proc.poll() is not None:
            if env.get('PULSE_SERVER') == TCP_BRIDGE:
                hint = 'load the bridge: pactl load-module module-native-protocol-tcp auth-anonymous=1 listen=127.0.0.1 port=4713'
            else:
                sock = env.get('PULSE_SERVER')
                hint = f'set PULSE_SERVER={sock} and start the daemon: pulseaudio --start --exit-idle-time=-1' if sock else 'start the daemon: pulseaudio --start --exit-idle-time=-1'
            raise RuntimeError(f'pulseaudio not reachable — {hint}')
        self.running = True
        self.label = label
        self.thread = threading.Thread(target=self._loop, daemon=True)
        self.thread.start()

    @staticmethod
    def _spawn(cmd, env):
        return subprocess.Popen(cmd, stdin=subprocess.PIPE,
                                stdout=subprocess.DEVNULL,
                                stderr=subprocess.DEVNULL, env=env)

    def _probe(self, cmd, env):
        try:
            p = self._spawn(cmd, env)
        except FileNotFoundError:
            return False
        time.sleep(0.8)
        if p.poll() is not None:
            return False
        return p

    def _loop(self):
        interval = self.block / self.engine.sr
        nxt = time.monotonic()
        while self.running:
            mono = self.engine.tick(self.block)
            try:
                self.proc.stdin.write(_to_stereo_bytes(mono))
                self.proc.stdin.flush()
            except (BrokenPipeError, OSError):
                break
            nxt += interval
            d = nxt - time.monotonic()
            if d > 0:
                time.sleep(d)

    def stop(self):
        self.running = False
        if self.thread:
            self.thread.join(timeout=2)
        if self.proc:
            try:
                self.proc.stdin.close()
            except Exception:
                pass
            try:
                self.proc.terminate()
            except Exception:
                pass


class NullSink:
    """silent sink: advances the clock at realtime pace, no audio (drift tests)."""

    def __init__(self, engine, block=512):
        self.engine = engine
        self.block = block
        self.thread = None
        self.running = False

    def start(self):
        self.running = True
        self.thread = threading.Thread(target=self._loop, daemon=True)
        self.thread.start()

    def _loop(self):
        interval = self.block / self.engine.sr
        nxt = time.monotonic()
        while self.running:
            self.engine.tick(self.block)
            nxt += interval
            d = nxt - time.monotonic()
            if d > 0:
                time.sleep(d)

    def stop(self):
        self.running = False
        if self.thread:
            self.thread.join(timeout=2)


class WavSink:
    """offline render: engine -> wav file. no audio device needed."""

    def __init__(self, engine, path, seconds):
        self.engine = engine
        self.path = path
        self.seconds = seconds

    def run(self):
        import wave
        total = int(self.engine.sr * self.seconds)
        chunks = []
        done = 0
        block = 4096
        while done < total:
            n = min(block, total - done)
            chunks.append(self.engine.tick(n))
            done += n
        audio = np.concatenate(chunks)
        data = np.clip(audio, -1.0, 1.0)
        with wave.open(self.path, 'wb') as w:
            w.setnchannels(1)
            w.setsampwidth(2)
            w.setframerate(self.engine.sr)
            w.writeframes((data * 32767.0).astype(np.int16).tobytes())
        return len(audio) / self.engine.sr


def make_live_sink(engine, block=512):
    return PacatSink(engine, block)
