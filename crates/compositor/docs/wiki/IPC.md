You can communicate with the running aether instance over an IPC socket.
Check `aether msg --help` for available commands.

The `--json` flag prints the response in JSON, rather than formatted.
For example, `aether msg --json outputs`.

> [!TIP]
> If you're getting parsing errors from `aether msg` after upgrading aether, make sure that you've restarted aether itself.
> You might be trying to run a newer `aether msg` against an older `aether` compositor.

### Event Stream

<sup>Since: 0.1.9</sup>

While most aether IPC requests return a single response, the event stream request will make aether continuously stream events into the IPC connection until it is closed.
This is useful for implementing various bars and indicators that update as soon as something happens, without continuous polling.

The event stream IPC is designed to give you the complete current state up-front, then follow up with updates to that state.
This way, your state can never "desync" from aether, and you don't need to make any other IPC information requests.

Where reasonable, event stream state updates are atomic, though this is not always the case.
For example, a window may end up with a workspace id for a workspace that had already been removed.
This can happen if the corresponding workspaces-changed event arrives before the corresponding window-changed event.

To get a taste of the events, run `aether msg event-stream`.
Though, this is more of a debug function than anything.
You can get raw events from `aether msg --json event-stream`, or by connecting to the aether socket and requesting an event stream manually.

You can find the full list of events along with documentation [here](https://aether-wm.github.io/aether/niri_ipc/enum.Event.html).

### Programmatic Access

`aether msg --json` is a thin wrapper over writing and reading to a socket.
When implementing more complex scripts and modules, you're encouraged to access the socket directly.

Connect to the UNIX domain socket located at `$NIRI_SOCKET` in the filesystem.
Write your request encoded in JSON on a single line, followed by a newline character, or by flushing and shutting down the write end of the connection.
Read the reply as JSON, also on a single line.

You can use `socat` to test communicating with aether directly:

```sh
$ socat STDIO "$NIRI_SOCKET"
"FocusedWindow"
{"Ok":{"FocusedWindow":{"id":12,"title":"t socat STDIO /run/u ~","app_id":"alacritty","workspace_id":6,"is_focused":true}}}
```

The reply is an `Ok` or an `Err` wrapping the same JSON object as you get from `aether msg --json`.

<sup>Since: next release</sup>
For more complex requests, you can pass `--print-request` to `aether msg` to see how the request should be formatted:

```sh
$ aether msg --print-request action focus-workspace 2
{"Action":{"FocusWorkspace":{"reference":{"Index":2}}}}
```

This is the format that you should use for communicating with the aether socket directly.
You can also use `aether msg raw-request` to send a raw JSON request without `socat`:

```sh
$ echo '{"Action":{"FocusWorkspace":{"reference":{"Id":8}}}}' | aether msg raw-request
"Handled"
```

You can find all available requests and response types in the [aether-ipc sub-crate documentation](https://aether-wm.github.io/aether/niri_ipc/).

### Backwards Compatibility

The JSON output *should* remain stable, as in:

- existing fields and enum variants should not be renamed
- non-optional existing fields should not be removed

However, new fields and enum variants will be added, so you should handle unknown fields or variants gracefully where reasonable.

The formatted/human-readable output (i.e. without `--json` flag) is **not** considered stable.
Please prefer the JSON output for scripts, since I reserve the right to make any changes to the human-readable output.

The `aether-ipc` sub-crate (like other aether sub-crates) is *not* API-stable in terms of the Rust semver; rather, it follows the version of aether itself.
In particular, new struct fields and enum variants will be added.
