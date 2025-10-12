require 'set'
require_relative 'graph'
require_relative 'search'

# Note: For visualization, we'll need to install the 'matplotlib' gem
begin
  require 'matplotlib/pyplot'
rescue LoadError
  puts "For visualization, please install matplotlib: gem install matplotlib"
end

class PathPlotter
  include LogExecutionTime

  def initialize(graph)
    @graph = graph
  end

  log_time def plot_path(path)
    plt = Matplotlib::Pyplot
    
    # Plot the path
    plt.plot(path.map { |v| v[0] }, path.map { |v| v[1] })
    
    # Plot the barriers
    @graph.barriers.each do |barrier|
      plt.plot(barrier.map { |v| v[0] }, barrier.map { |v| v[1] })
    end
    
    plt.xlim(-1, 8)
    plt.ylim(-1, 8)
    plt.show
  end
end

def main
  graph = AStarGraph.new
  search = AStarSearch.new(graph)
  
  start_pos = [0, 0]
  goal_pos = [7, 7]
  
  result, cost = search.search(start_pos, goal_pos)
  puts "Route: #{result}"
  puts "Cost: #{cost}"
  
  # Only attempt to plot if matplotlib is available
  if defined?(Matplotlib)
    plotter = PathPlotter.new(graph)
    plotter.plot_path(result)
  end
end

if __FILE__ == $PROGRAM_NAME
  main
end
