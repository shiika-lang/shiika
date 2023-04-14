#!/bin/bash 
set -e

cargo run -- run examples/fib.sk > examples/fib.actual.txt
diff examples/fib.actual.txt examples/fib.expected_out.txt

cargo run -- run examples/hello.sk > examples/hello.actual.txt
diff examples/hello.actual.txt examples/hello.expected_out.txt

cargo run -- run examples/lifegame.sk > examples/lifegame.actual.txt
diff examples/lifegame.actual.txt examples/lifegame.expected_out.txt

cargo run -- run examples/mandel.sk > examples/mandel.actual.pbm
diff examples/mandel.actual.pbm examples/mandel.expected_out.pbm

cargo run -- run examples/ray.sk > examples/ray.actual.ppm
diff examples/ray.actual.ppm examples/ray.expected_out.ppm
