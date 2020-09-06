#
# Rakefile
#
# Basically you don't need to run this. Miscellaneous tasks

task :doc do
  chdidr "doc/shg" do
    sh "mdbook build"
  end
end

desc "git ci, git tag and git push"
task :release do
  sh "git diff --cached"
  v = File.read('CHANGELOG.md')[/v([\d\.]+) /]
  puts "release as #{v}? [y/N]"
  break unless $stdin.gets.chomp == "y"

  sh "git ci -m '#{v}'"
  sh "git tag '#{v}'"
  sh "git push origin master --tags"
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
