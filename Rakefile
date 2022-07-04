#
# Rakefile
#
# Basically you don't need to run this. Miscellaneous tasks
require 'yaml'

task :doc do
  chdidr "doc/shg" do
    sh "mdbook build"
  end
end

desc "git ci, git tag and git push"
task :release do
  sh "git diff --cached"
  v = "v" + YAML.load_file("src/cli.yml")["version"]
  puts "release as #{v}? [y/N]"
  break unless $stdin.gets.chomp == "y"

  sh "git ci -m '#{v}'"
  sh "git tag '#{v}'"
  sh "git push origin main --tags"
end

task :default => :test

task :test do
  cd "lib/skc_rustlib" do
    sh "cargo build"
  end
  sh "cargo run -- build-corelib"
  sh "cargo test -- --nocapture"
end

RUST_FILES = Dir["lib/**/*.rs"] + Dir["src/*.rs"]
RUSTLIB_SIG = "lib/skc_rustlib/provided_methods.json5"

RUSTLIB_FILES = [
  *Dir["lib/skc_rustlib/src/**/*.rs"],
  RUSTLIB_SIG,
  "lib/skc_rustlib/Cargo.toml",
]
RUSTLIB_A = "lib/skc_rustlib/target/debug/libskc_rustlib.a"
file RUSTLIB_A => RUSTLIB_FILES do
  cd "lib/skc_rustlib" do
    #sh "cargo fmt"
    sh "cargo build"
  end
end

BUILTIN_BC = "builtin/builtin.bc"
file BUILTIN_BC => [*RUST_FILES, RUSTLIB_SIG, *Dir["builtin/*.sk"]] do
  sh "cargo run -- build-corelib"
end

A_OUT = "./a.sk.out"
file A_OUT => [*RUST_FILES, RUSTLIB_A, BUILTIN_BC, "./a.sk"] do
  #sh "cargo fmt"
  sh "cargo run -- run ./a.sk"
end

task :fmt do
  sh "cargo fmt"
end

task :asm do
  sh "llc ./a.sk.ll"
end

#
# debugify
#

A_BC = "./a.sk.bc"
file A_BC => RUST_FILES + [BUILTIN_BC, RUSTLIB_A, "./a.sk"] do
  sh "cargo run -- run ./a.sk"
end
A_LL = "./a.sk.ll"
file A_LL => RUST_FILES + [BUILTIN_BC, RUSTLIB_A, "./a.sk"] do
  sh "cargo run -- run ./a.ll"
end

DEBUG_LL = "./a.sk.debug.ll"
file DEBUG_LL => A_LL do
  sh "opt #{A_LL} -debugify -S -o #{DEBUG_LL}"
end

DEBUG_OUT = "./a.debug.out"
file DEBUG_OUT => [A_BC, BUILTIN_BC, RUSTLIB_A, DEBUG_LL] do
  sh "clang",
    "-lm",
    "-ldl",
    "-lpthread",
    "-o", DEBUG_OUT,
    BUILTIN_BC,
    RUSTLIB_A,
    SK_LL
end

task :debugify => DEBUG_OUT

#task :a => [:fmt, A_OUT]
task :a => [:fmt] do
#   sh "cargo run -- run a.sk"
  sh "cargo run -- run ~/proj/BidirectionalTypechecking/bidi.sk"
end

