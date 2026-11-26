// NSH's browser logger already writes to the console. Mirror its application
// records into the Rust log viewer without replacing the logger or the console.
export function createHost(player) {
  // Only a browser player has log options; the headless test host has none.
  if (player.options) {
    const previous = player.options.onLog;
    player.options.onLog = (level, message) => {
      previous?.call(player.options, level, message);
      if (message.startsWith("dzip-gui: ")) {
        // Logs can arrive during a WASM render. Deliver the event after it
        // unwinds, without re-entering the same engine operation.
        queueMicrotask(() => player.hostEvent("dzip.log", { level, message }));
      }
    };
  }
  // NSH requires a call handler on a Web host; this host offers only events.
  return { call(name) { throw new Error(`Unsupported host call: ${name}`); } };
}
