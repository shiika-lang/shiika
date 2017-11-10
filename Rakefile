require 'bundler/setup'

file 'lib/shiika/parser.ry' => 'lib/shiika/parser.ry.erb' do
  sh "erb lib/shiika/parser.ry.erb > lib/shiika/parser.ry"
end

file 'lib/shiika/parser.rb' => 'lib/shiika/parser.ry' do
  cmd = "racc -o lib/shiika/parser.rb lib/shiika/parser.ry"
  cmd.sub!("racc", "racc --debug") if ENV["DEBUG"] == "1"
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

task :default => [:parser, :test]
