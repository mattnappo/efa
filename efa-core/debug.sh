#!/bin/bash

rm -f asm dis dis1 dis2 test.db dis.db

#cargo run run examples/call.asm test.db > asm
#cargo run run asm dis.db
#cargo run dis dis.db > dis

cargo run run examples/call.asm test.db
cargo run dis test.db > dis1

cargo run run dis1 dis.db
cargo run dis dis.db > dis2

