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
  cd "doc/shg" do
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

CARGO_TARGET = ENV["SHIIKA_CARGO_TARGET"] || "./target"

task :fmt do
  sh "cargo fmt"
end

task :asm do
  sh "llc ./a.sk.ll"
end

task :a => :async

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
# async runtime
#
task :async do
  sh "cargo fmt"
  sh "cargo run -- build packages/core"
  sh "cargo run -- compile a.sk"
end
task async_test: :async do
  sh "./a.out"
end
task :async_integration_test do
  bin = File.join(CARGO_TARGET, "debug/shiika")
  sh "cargo build"
  sh "#{bin} build packages/core"
  Dir["tests/new_runtime/*.sk"].each do |path|
    next if ENV["FILTER"] && !path.include?(ENV["FILTER"])
    name = path.sub(".sk", "")
    sh "#{bin} compile #{name}.sk"
    puts "--"
    output = nil
    Timeout.timeout(5) do
      output = `#{name}.out 2>&1`
    end
    exit_status = $?.exitstatus
    puts output
    puts "---"
    if output.strip == "ok" && exit_status == 0
      # pass
    elsif exit_status != 0
      raise "Test failed with exit status #{exit_status}: #{path}\n#{output}"
    elsif output.include?("ng:")
      raise "Test failed: #{path}\n#{output}"
    else
      raise "Test did not output 'ok': #{path}\n#{output}"
    end
  end
end
task :compat do
  bin = File.join(CARGO_TARGET, "debug/shiika")
  sh "cargo build"
  log = File.open("compat_test.log", "w")
  filter = ENV["FILTER"]; filter = nil if filter.to_s.strip.empty?
  Dir["tests/compat/*.sk"].each do |path|
    $stderr.puts "Testing #{path}..."
    log.puts "--- Testing #{path} ---"
    next if filter && !path.include?(filter)
    name = path.sub(".sk", "")
    compile_output = `#{bin} compile #{name}.sk 2>&1`
    log.puts compile_output
    next unless $?.success?
    output = nil
    Timeout.timeout(5) do
      output = `#{name}.out 2>&1`
    end
    exit_status = $?.exitstatus
    log.puts output
    if output.strip == "ok" && exit_status == 0
      # pass
    elsif exit_status != 0
      log.puts "Test failed with exit status #{exit_status}: #{path}\n#{output}"
    elsif output.include?("ng:")
      log.puts "Test failed: #{path}\n#{output}"
    else
      log.puts "Test did not output 'ok': #{path}\n#{output}"
    end
  end
  log.close
end

#
# debugging
#

task :segv do
  bin = ENV["BIN"] || "./a.out"
  sh "lldb #{bin} -o run -o bt -o exit > a.dump.txt"
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

task :lldb_mcp do
  sh "lldb", "-o", "protocol-server start MCP listen://localhost:59999"
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
