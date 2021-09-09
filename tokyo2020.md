# rs_toio_cube

Access test to toio core cube with Rust on Windows

## Getting Started

### Prerequisites

You pair 2 toio core cubes with your PC before running this sample code.  
This sample uses a toio mat.

Supported mats:
* toio collection mat
* gesundroid mat

### How to run

```
git clone https://github.com/kaz399/rs_toio_cube.git
cargo build --example tokyo2020
cargo run --example tokyo2020 --mat (MAT TYPE) --cube 2
```

#### Options

`--mat MAT_TYPE` :  specify mat

| MAT_TYPE | description |
|----------| ----------- |
| tc1      | toio collection mat (wring side) |
| tc2      | toio collection mat (colored tiles side) |
| gesun    | gesundroid mat |

`--cube n` : Number of cubes to control.  

Specify between 1 and 4.
(If you want to specify `4`, you must register 4 cubes to windows OS in advance.)

## Notice

**Don't replace** the bluetooth driver to WinUSB.  
If you had replaced the bluetooth driver to WinUSB already, You have to revert to original driver. (WinUSB is required by [toio.js](https://github.com/toio/toio.js/))


## Reference

[toio Core Cube Specification](https://toio.github.io/toio-spec/)

## License

3-Clause BSD License
