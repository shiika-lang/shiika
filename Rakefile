require 'bundler/setup'

file 'lib/shiika/parser.ry' => 'lib/shiika/parser.ry.erb' do
  sh "erb lib/shiika/parser.ry.erb > lib/shiika/parser.ry"
end

file 'lib/shiika/parser.rb' => 'lib/shiika/parser.ry' do
  debug = (ENV["DEBUG"] == "1")
  cmd = "racc #{'--verbose --debug' if debug} -o lib/shiika/parser.rb lib/shiika/parser.ry"
  sh cmd
end

desc "run test"
task :test do
  if ENV["F"]
    sh "rspec --fail-fast"
  else
    sh "rspec"
  end
end

task :parser => 'lib/shiika/parser.rb'

task :doc do
  sh "gitbook build book doc"
end

#require_relative 'lib/shiika/version'
desc "git ci, git tag and git push"
task :release do
  sh "git diff HEAD"
  v = "v0.2.1"
  puts "release as #{v}? [y/N]"
  break unless $stdin.gets.chomp == "y"

  sh "git ci -am '#{v}'"
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
