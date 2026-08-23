const { spawn } = require("child_process");
const fs2 = require("fs");
const log = fs2.openSync("logs/sleeper.log", "a");
const child = spawn("py", ["-3", "scripts/autonomous_sleeper.py"], {
  cwd: "C:/ayesha-os",
  detached: true,
  stdio: ["ignore", "pipe", "pipe"]
});
child.stdout.on("data", d => fs2.writeSync(log, d));
child.stderr.on("data", d => fs2.writeSync(log, d));
child.on("spawn", () => console.log("sleeper started, pid=" + child.pid));
child.unref();
setTimeout(() => process.exit(0), 1000);
