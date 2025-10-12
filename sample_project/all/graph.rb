require_relative 'decorators'

module CostStrategy
  BARRIER = :barrier
  DISTANCE = :distance
  COMBINED = :combined
end

class GraphBase
end

class AStarGraph < GraphBase
  include LogExecutionTime
  
  attr_reader :barriers

  def initialize
    @barriers = []
    @barriers << [
      [2, 4], [2, 5], [2, 6],
      [3, 6], [4, 6], [5, 6],
      [5, 5], [5, 4], [5, 3],
      [5, 2], [4, 2], [3, 2]
    ]
  end

  private def barrier_cost(a, b)
    @barriers.any? { |barrier| barrier.include?(b) } ? 100 : 1
  end

  private def distance_cost(a, b)
    (b[0] - a[0]).abs + (b[1] - a[1]).abs
  end

  private def combined_cost(a, b)
    barrier_cost(a, b) * distance_cost(a, b)
  end

  def move_cost(a, b, strategy: CostStrategy::BARRIER)
    case strategy
    when CostStrategy::BARRIER
      barrier_cost(a, b)
    when CostStrategy::DISTANCE
      distance_cost(a, b)
    when CostStrategy::COMBINED
      combined_cost(a, b)
    else
      raise ArgumentError, "Unknown cost strategy: #{strategy}"
    end
  end

  log_time def heuristic(start, goal)
    d = 1
    d2 = 1
    dx = (start[0] - goal[0]).abs
    dy = (start[1] - goal[1]).abs
    d * (dx + dy) + (d2 - 2 * d) * [dx, dy].min
  end

  log_time def get_vertex_neighbours(pos, cost_strategy: CostStrategy::BARRIER)
    neighbors = []
    [
      [1, 0], [-1, 0], [0, 1], [0, -1],
      [1, 1], [-1, 1], [1, -1], [-1, -1]
    ].each do |dx, dy|
      x2 = pos[0] + dx
      y2 = pos[1] + dy
      next if x2 < 0 || x2 > 7 || y2 < 0 || y2 > 7
      
      neighbor = [x2, y2]
      neighbors << neighbor if move_cost(pos, neighbor, strategy: cost_strategy) < 100
    end
    neighbors
  end
end
