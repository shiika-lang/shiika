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
  v = "v" + YAML.load_file("cli.yml")["version"]
  puts "release as #{v}? [y/N]"
  break unless $stdin.gets.chomp == "y"

  sh "git ci -m '#{v}'"
  sh "git tag '#{v}'"
  sh "git push origin main --tags"
end

task :default => :test

task :run do
  sh "cargo run"
  sh "llc a.ll"
  sh "cc -I/usr/local/Cellar/bdw-gc/7.6.0/include/ -L/usr/local/Cellar/bdw-gc/7.6.0/lib/ -lgc -o a.out a.s"
  sh "./a.out"
end

task :opt do
  sh "cargo run"
  sh "opt -O3 a.ll > a.bc"
  sh "llvm-dis a.bc -o a2.ll"
  sh "llc a.bc"
  sh "cc -I/usr/local/Cellar/bdw-gc/7.6.0/include/ -L/usr/local/Cellar/bdw-gc/7.6.0/lib/ -lgc -o a.out a.s"
  sh "./a.out"
end

task :build do
  sh "cargo build"
end

task :clean do
  files = `git status -sz --untracked-files=normal --ignored`.
            lines("\0", chomp: true).
            filter_map { |l| /\A!! /.match(l)&.post_match }
  rm_rf files
end

task :test do
  sh "cargo test"
end

task :tmp do
  mode = "debug"
  cd "src/rustlib" do
    sh "cargo build #{mode == 'debug' ? '' : '--'+mode}"
  end
  name = "a"
  #sh "llvm-as builtin/builtin.ll"
  sh "llvm-as examples/#{name}.sk.ll"
  sh "clang" +
    " -target x86_64-apple-macosx10.15.7" +
    " -L/usr/local/opt/bdw-gc/lib/" +
    " -lgc" +
    " -o #{name}.out" +
    " ~/tmp/shiika/#{mode}/librustlib.a" +
    " examples/#{name}.sk.bc builtin/builtin.bc"
  sh "./#{name}.out"
end

task :tmp2 do
  cd "src/rustlib" do
    sh "cargo fmt"
    sh "cargo build"
  end
  sh "cargo run -- build_corelib"
  sh "cargo run -- run examples/a.sk"
#  #sh "llvm-as examples/#{name}.sk.ll"
#  sh "clang" +
#    " -target x86_64-apple-macosx10.15.7" +
#    " -o #{name}.out" +
#    " ~/tmp/shiika/debug/librustlib.a" +
#    " examples/#{name}.sk.bc builtin/builtin.bc"
#  sh "./#{name}.out"
end
