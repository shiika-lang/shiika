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
RUSTLIB = "src/rustlib/target/debug/librustlib.a"
file RUSTLIB => RUSTLIB_FILES do
  cd "src/rustlib" do
    sh "cargo fmt"
    sh "cargo build"
  end
end

BUILTIN = "builtin/builtin.bc"
file BUILTIN => RUST_FILES + Dir["builtin/*.sk"] do
  sh "cargo run -- build_corelib"
end

A_OUT = "examples/a.sk.out"
file A_OUT => RUST_FILES + [BUILTIN, RUSTLIB, "examples/a.sk"] do
  sh "cargo run -- run examples/a.sk"
end

task :a => A_OUT
