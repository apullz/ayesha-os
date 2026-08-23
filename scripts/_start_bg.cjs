const { spawn } = require("child_process");
const fs = require("fs");
const log = fs.openSync("logs/sleeper.log", "a");
const child = spawn("py", ["-3", "scripts/autonomous_sleeper.py"], {
  cwd: "C:/ayesha-os",
  detached: true,
  stdio: ["ignore", "pipe", "pipe"]
});
child.stdout.on("data", d => fs.writeSync(log, d));
child.stderr.on("data", d => fs.writeSync(log, d));
child.on("spawn", () => {
  console.log("sleeper started, pid=" + child.pid);
  setTimeout(() => process.exit(0), 500);
});
child.on("error", e => {
  console.log("spawn error: " + e.message);
  process.exit(1);
});
child.unref();
