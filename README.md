# rs_toio_cube

Access test to toio core cube with Rust on Windows

## Getting Started

### Prerequisites

You pair toio core cube(s) with your PC before running this sample code.

### How to run

```
git clone https://github.com/kaz399/rs_toio_cube.git
cd rs_toio_cube/core_cube
cargo test
```

## Notice

**Don't replace** the bluetooth driver to WinUSB.  
If you had replaced the bluetooth driver to WinUSB already, You have to revert to original driver. (WinUSB is required by [toio.js](https://github.com/toio/toio.js/))


## Reference

[toio Core Cube Specification](https://toio.github.io/toio-spec/)

## License

3-Clause BSD License
