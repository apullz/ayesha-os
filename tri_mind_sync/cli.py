"""
tri-mind sync CLI
unified command-line interface for managing sync
"""

import sys
import json
import argparse

from .engine import TriMindEngine
from .config import TriMindConfig


def main():
    parser = argparse.ArgumentParser(
        prog="tri-mind",
        description="tri-mind sync - bidirectional sync for ayesha-os, github, huggingface",
    )
    parser.add_argument("--config", "-c", help="path to config file")
    parser.add_argument("--base-dir", "-d", help="base directory to sync")

    sub = parser.add_subparsers(dest="command", help="command to run")

    sub.add_parser("status", help="show sync status")

    sync_cmd = sub.add_parser("sync", help="run sync")
    sync_cmd.add_argument("--target", choices=["all", "local", "github", "huggingface"],
                          default="all")

    push_cmd = sub.add_parser("push", help="push to github")
    push_cmd.add_argument("--message", "-m", default="tri-mind sync")
    push_cmd.add_argument("files", nargs="*")

    pull_cmd = sub.add_parser("pull", help="pull from github")
    pull_cmd.add_argument("files", nargs="*")

    watch_cmd = sub.add_parser("watch", help="start continuous sync loop")
    watch_cmd.add_argument("--github-interval", type=int, default=300)
    watch_cmd.add_argument("--hf-interval", type=int, default=60)

    sub.add_parser("conflicts", help="show unresolved conflicts")
    sub.add_parser("init", help="create default config file")
    sub.add_parser("scan", help="scan local directory for changes")

    args = parser.parse_args()
    if not args.command:
        parser.print_help()
        return

    config = TriMindConfig.load(args.config)
    base_dir = args.base_dir or str(config.get_base_dir())
    engine = TriMindEngine(config=config, base_dir=base_dir)

    if args.command == "init":
        config.save("tri_mind_sync.json")
        print("created tri_mind_sync.json - edit to set tokens and repos")

    elif args.command == "status":
        engine.print_status()

    elif args.command == "sync":
        if args.target == "all":
            print(json.dumps(engine.sync_all(), indent=2))
        elif args.target == "local":
            for e in engine.sync_local(): print(f"  {e.status.value}: {e.message}")
        elif args.target == "github":
            print(json.dumps(engine.sync_github(), indent=2))
        elif args.target == "huggingface":
            print(json.dumps(engine.sync_huggingface(), indent=2))

    elif args.command == "push":
        if args.files:
            for f in args.files:
                engine.github.push_file(f, args.message)
                print(f"  pushed: {f}")
        else:
            ok = engine.github.push(message=args.message)
            print("push ok" if ok else "push failed")

    elif args.command == "pull":
        if args.files:
            for f in args.files:
                r = engine.github.pull_file(f)
                print(f"  pulled: {f}" if r else f"  failed: {f}")
        else:
            for e in engine.github.pull():
                print(f"  {e.status.value}: {e.message}")

    elif args.command == "watch":
        print("tri-mind sync loop started (Ctrl+C to stop)")
        engine.start_loop(args.github_interval, args.hf_interval)
        try:
            while True: time.sleep(1)
        except KeyboardInterrupt:
            engine.stop_loop()
            print("\nstopped")

    elif args.command == "conflicts":
        conflicts = engine.resolver.get_unresolved_conflicts()
        if not conflicts: print("no conflicts")
        else:
            for c in conflicts: print(f"  {c.file_path} (vs {c.remote_source.value})")

    elif args.command == "scan":
        events = engine.sync_local()
        if not events: print("no changes")
        else:
            for e in events: print(f"  {e.status.value}: {e.message}")


if __name__ == "__main__":
    main()
