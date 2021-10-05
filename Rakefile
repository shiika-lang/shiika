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
  cd "src/rustlib" do
    sh "cargo build"
  end
  sh "cargo run -- build_corelib"
  sh "cargo test"
end

RUST_FILES = Dir["src/**/*.rs"]

RUSTLIB_FILES = Dir["src/rustlib/src/**/*.rs"] + ["src/rustlib/Cargo.toml"]
RUSTLIB_A = "src/rustlib/target/debug/librustlib.a"
file RUSTLIB_A => RUSTLIB_FILES do
  cd "src/rustlib" do
    sh "cargo fmt"
    sh "cargo build"
  end
end

BUILTIN_BC = "builtin/builtin.bc"
file BUILTIN_BC => RUST_FILES + Dir["builtin/*.sk"] do
  sh "cargo run -- build_corelib"
end

A_OUT = "examples/a.sk.out"
file A_OUT => RUST_FILES + [BUILTIN_BC, RUSTLIB_A, "examples/a.sk"] do
  sh "cargo fmt"
  sh "cargo run -- run examples/a.sk"
end

task :a => A_OUT

task :asm do
  sh "llc examples/a.sk.ll"
end

#
# debugify
#

A_BC = "examples/a.sk.bc"
file A_BC => RUST_FILES + [BUILTIN_BC, RUSTLIB_A, "examples/a.sk"] do
  sh "cargo run -- run examples/a.sk"
end
A_LL = "examples/a.sk.ll"
file A_LL => RUST_FILES + [BUILTIN_BC, RUSTLIB_A, "examples/a.sk"] do
  sh "cargo run -- run examples/a.ll"
end

DEBUG_LL = "examples/a.sk.debug.ll"
file DEBUG_LL => A_LL do
  sh "opt #{A_LL} -debugify -S -o #{DEBUG_LL}"
end

DEBUG_OUT = "examples/a.debug.out"
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
