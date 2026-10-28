# python-client

PyO3 bindings that let Python consumers receive frames from a `control-hub`. The crate is
`device-system-py` and produces the extension module `device_system` (`src/lib.rs`). The bindings
wrap `access-client::AccessClient`, so Python uses the same wire decoder as the Rust client.

> **Status:** the bindings are implemented, packaged with maturin (`pyproject.toml`), and covered by
> a pytest that runs the loopback example. See `SPECS.md` §5 and §8.

## Build

From the workspace root (system libraries as described in the top-level `README.md`):

```bash
cargo build -p device-system-py
```

Cargo produces `target/debug/libdevice_system.so`. Python imports the module by its file name, so
copy it to `device_system.so` on a directory that is on your Python path:

```bash
mkdir -p /tmp/device-system-py
cp target/debug/libdevice_system.so /tmp/device-system-py/device_system.so
PYTHONPATH=/tmp/device-system-py python3 -c "import device_system"
```

## Usage

Start a hub (`cargo run -p control-hub`) and a producer (see the top-level `README.md`), then:

```python
from device_system import DeviceSystem

ds = DeviceSystem("127.0.0.1", 5556)
for frame in ds.get_frame():
    print(frame.device_id, frame.device_type, frame.frame_shape, len(frame.data))
```

`DeviceSystem(hub_ip, hub_port)` connects to the hub's XPUB endpoint and raises `ValueError` for an
invalid IPv4 address and `RuntimeError` if the connection fails.

## API

### `DeviceSystem(hub_ip: str, hub_port: int)`

| Member | Behaviour |
|---|---|
| `next_frame(timeout_ms=1000)` | Returns a `Frame`, or `None` if none arrives within the timeout. |
| `get_frame()` | Returns the object itself so it can be used as an iterator. |
| `__next__` | Blocks until the next frame. It checks for Ctrl+C about every 100 ms. |
| `close()` | Disconnects from the hub. |

`DeviceSystem` is not thread-safe. Use it only from the thread that created it.

### `Frame`

| Attribute | Type | Meaning |
|---|---|---|
| `device_id` | `str` | Device path or identifier reported by the producer. |
| `device_name` | `str` | Human-readable device name. |
| `device_type` | `str` | `"camera"` or `"microphone"`. |
| `frame_shape` | `list[int]` | `[width, height]` for cameras, `[buffer_size, channels]` for microphones. |
| `rx_timestamp` | `float \| None` | Receive time, seconds since the Unix epoch. |
| `tx_timestamp` | `float \| None` | Send time set by the producer, seconds since the Unix epoch. |
| `data` | `bytes` | Raw payload. Cameras use the negotiated FourCC (default `YUYV`); microphones use interleaved `S16LE`. |

Microphone `data` is interleaved little-endian signed 16-bit samples, which `array('h')` or
`struct` can decode.

## Limitations

- The package is built from source with maturin; no prebuilt wheels are published yet.
- The pytest needs `cargo` on the path and runs the loopback example, so it is a local integration test.
- Frames from several producers arrive interleaved. Use `device_id` to tell them apart; the bindings
  do not sync them.
