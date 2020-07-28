require 'bundler/setup'

file 'lib/shiika/parser.ry' => 'lib/shiika/parser.ry.erb' do
  sh "erb lib/shiika/parser.ry.erb > lib/shiika/parser.ry"
end

file 'lib/shiika/parser.rb' => 'lib/shiika/parser.ry' do
  debug = (ENV["DEBUG"] == "1")
  cmd = "racc #{'--verbose --debug' if debug} -o lib/shiika/parser.rb lib/shiika/parser.ry"
  sh cmd
end

task :parser => 'lib/shiika/parser.rb'

task :doc do
  chdidr "doc/shg" do
    sh "mdbook build"
  end
end

#require_relative 'lib/shiika/version'
desc "git ci, git tag and git push"
task :release do
  sh "git diff --cached"
  v = File.read('CHANGELOG.md')[/v([\d\.]+) /, 1]
  puts "release as #{v}? [y/N]"
  break unless $stdin.gets.chomp == "y"

  sh "git ci -m '#{v}'"
  sh "git tag '#{v}'"
  sh "git push origin master --tags"
end

task :default => [:parser, :test]

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

rule ".rs" => ".rs.erb" do |t|
  sh "erb #{t.source} > #{t.name}"
end

LIBS = [
  "src/corelib/int.rs",
  "src/corelib/float.rs",
]

task :build => LIBS do
  sh "cargo build"
end

task :clean do
  files = `git status -sz --untracked-files=normal --ignored`.
            lines("\0", chomp: true).
            filter_map { |l| /\A!! /.match(l)&.post_match }
  rm_rf files
end

task :test => LIBS do
  sh "cargo test"
end
