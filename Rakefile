#
# Rakefile
#
# Basically you don't need to run this. Miscellaneous tasks

require "timeout"

if File.exist?(".env")
  File.readlines(".env").each do |line|
    l, r = line.split("=", 2)
    ENV[l.strip] = r.strip if l && r
  end
end

task :doc do
  chdidr "doc/shg" do
    sh "mdbook build"
  end
end

desc "git ci, git tag and git push"
task :release do
  ver = File.read('CHANGELOG.md')[/v([\d\.]+) /, 1]
  v = "v" + ver
  raise "Cargo.toml not updated" unless File.readlines("Cargo.toml").include?("version = \"#{ver}\"\n")
  sh "git diff --cached"
  puts "release as #{v}? [y/N]"
  break unless $stdin.gets.chomp == "y"

  sh "git ci -m '#{v}'"
  sh "git tag '#{v}'"
  sh "git push origin main --tags"
end

task :default => :test

task :compile do
  cd "lib/skc_rustlib" do
    sh "cargo build"
  end
  sh "cargo run -- build-corelib"
end

task :test do
  sh "cargo fmt"
  cd "lib/skc_rustlib" do
    sh "cargo build"
  end
  sh "cargo run -- build-corelib"
  sh "cargo test -- --nocapture"
end

desc "Test if examples/*.sk runs as expected"
task :release_test do
  Dir["examples/*.expected_out.*"].each do |exp|
    next if ENV["FILTER"] && !exp.include?(ENV["FILTER"])
    exp =~ %r{examples/(.*)\.expected_out\.(.*)} or raise
    name, ext = $1, $2
    actual = "examples/#{name}.actual.#{ext}"
    sh "cargo run -- run examples/#{name}.sk > #{actual}"
    if File.read(actual) != File.read(exp)
      sh "diff #{exp} #{actual}"
      raise "release_test failed for #{name}.sk"
    end
  end
end

task :llvm do
  cd "lib/skc_rustlib" do
    sh "cargo rustc -- --emit=llvm-ir -C debuginfo=0 -C opt-level=3 "
  end
  # ~/tmp/cargo_target/debug/deps/
end

RUST_FILES = Dir["lib/**/*.rs"] + Dir["src/*.rs"]
RUSTLIB_SIG = "lib/skc_rustlib/provided_methods.json5"

RUSTLIB_FILES = [
  *Dir["lib/skc_rustlib/src/**/*.rs"],
  RUSTLIB_SIG,
  "lib/skc_rustlib/Cargo.toml",
]
CARGO_TARGET = ENV["SHIIKA_CARGO_TARGET"] || "./target"
RUSTLIB_A = File.expand_path "#{CARGO_TARGET}/debug/libskc_rustlib.a"
file RUSTLIB_A => RUSTLIB_FILES do
  cd "lib/skc_rustlib" do
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
  sh "cargo run -- compile ./a.sk"
end
A_LL = "./a.sk.ll"
file A_LL => RUST_FILES + [BUILTIN_BC, RUSTLIB_A, "./a.sk"] do
  sh "cargo run -- compile ./a.sk"
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
    "-O0",
    BUILTIN_BC,
    RUSTLIB_A,
    DEBUG_LL
end

task "clang" do
  sh "clang",
    "-lm",
    "-ldl",
    "-lpthread",
    "-o", "a.sk.out",
    "-framework", "Foundation",
    "-O0",
    BUILTIN_BC,
    RUSTLIB_A,
    "a.sk.ll"
end

task :debugify => DEBUG_OUT

task :a => :async
#task :a => [:fmt, A_OUT] do
#task :a => [:fmt] do
  # sh "cargo clippy"
  # sh "cargo run -- run a.sk"
#end

#
# git worktree
#
task :worktree_add do
  name = ENV.fetch("NAME")
  dir = ",/worktrees/#{name}"
  sh "git worktree add #{dir} origin/main"

  mkdir "#{dir}/.cargo"
  File.write("#{dir}/.cargo/config.toml", 
             "build.target-dir = \"~/tmp/cargo_targets/#{name}\"")
  File.write("#{dir}/.env", <<~EOD)
SHIIKA_CARGO_TARGET=~/tmp/cargo_targets/#{name}
SHIIKA_ROOT=~/proj/shiika/#{dir}
SHIIKA_WORK=~/.shiika/
  EOD
end

#
# new async runtime
#
task :async do
  sh "cargo fmt"
  sh "cargo run --bin exp_shiika --features new-runtime -- build packages/core"
  sh "cargo run --bin exp_shiika --features new-runtime -- compile a.sk"
end
task async_test: :async do
  sh "./a.out"
end
task :async_integration_test do
  sh "cargo run --bin exp_shiika --features new-runtime -- build packages/core"
  Dir["tests/new_runtime/*.sk"].each do |path|
    next if ENV["FILTER"] && !path.include?(ENV["FILTER"])
    name = path.sub(".sk", "")
    sh "cargo run --bin exp_shiika --features new-runtime -- compile #{name}.sk"
    puts "--"
    Timeout.timeout(5) do
      sh "#{name}.out > #{name}.actual_out 2>&1"
    end
    puts "---"
    sh "diff #{name}.actual_out #{name}.expected_out"
  end
end

#
# debugging
#

task :coredump do
  sh "lldb ./a.out -o run -o bt -o exit > a.dump.txt"
end

task :err do
  sh "rake async_test > err.txt 2>&1"
end

task :lldb do
  sh "rake tmp"
  sh "lldb", "a.out",
    "-o", "breakpoint set -f a.sk -l 1",
    #"-o", "run",
    #"-o", "register read"
    ""
end

task :tmp do
  sh "clang-18 -v -lm -o a.out a.ll ~/.shiika/packages/core-0.1.0/cargo_target/debug/libext.a ~/.shiika/packages/core-0.1.0/lib/index.ll -ldl -lpthread"
end
=begin
source_filename = "a.ll"
!llvm.dbg.cu = !{!0}
!llvm.module.flags = !{!6}
!llvm.ident = !{!7}
!0 = distinct !DICompileUnit(language: DW_LANG_C, file: !1, producer: "hand-written", isOptimized: false, emissionKind: FullDebug, enums: !2)
!1 = !DIFile(filename: "a.ll", directory: ".")
!2 = !{}
!3 = distinct !DISubprogram(name: "main", file: !1, line: 1, type: !2)
!4 = !DILocation(line: 77, column: 1, scope: !3)
!5 = !DILocation(line: 1, column: 1, scope: !3)
!6 = !{i32 2, !"Debug Info Version", i32 3}
!7 = !{!"handwritten"}
=end
