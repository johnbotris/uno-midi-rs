for now requires nightly rust 2021-01-07 to build:

`rustup override set nightly-2021-01-07`


### notes for self

##### 1
in Cargo.toml:

> \# Workaround to fix compiler panics  
> \# https://github.com/Rahix/avr-hal/issues/131  
> [profile.dev.package.compiler_builtins]  
> overflow-checks = false

##### 2

can't `%` with >=32bit integers because of https://github.com/rust-lang/rust/issues/82242

so I guess we just use u16/i16

##### 3

if avrdude can't find the arduino serial port change `$SERIAL` in `uno-runner.sh` to either `/dev/ttyACM0` or `/dev/ttyACM1`
