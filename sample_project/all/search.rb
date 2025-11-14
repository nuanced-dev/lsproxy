require 'set'
require_relative 'decorators'
require_relative 'graph'

class AStarSearch
  include LogExecutionTime

  def initialize(graph)
    @graph = graph
  end

  log_time def initialize_search(start, goal)
    {
      g_score: { start => 0 },
      f_score: { start => @graph.heuristic(start, goal) },
      closed_vertices: Set.new,
      open_vertices: Set.new([start]),
      came_from: {}
    }
  end

  private def reconstruct_path(current, came_from, start)
    path = []
    while came_from.key?(current)
      path << current
      current = came_from[current]
    end
    path << start
    path.reverse
  end

  log_time def search(start, goal)
    search_state = initialize_search(start, goal)
    g_score = search_state[:g_score]
    f_score = search_state[:f_score]
    closed_vertices = search_state[:closed_vertices]
    open_vertices = search_state[:open_vertices]
    came_from = search_state[:came_from]

    while open_vertices.any?
      current = open_vertices.min_by { |pos| f_score[pos] }
      return [reconstruct_path(current, came_from, start), f_score[current]] if current == goal

      open_vertices.delete(current)
      closed_vertices.add(current)

      @graph.get_vertex_neighbours(current).each do |neighbor|
        next if closed_vertices.include?(neighbor)

        candidate_g = g_score[current] + @graph.move_cost(current, neighbor)

        if !open_vertices.include?(neighbor)
          open_vertices.add(neighbor)
        elsif candidate_g >= (g_score[neighbor] || Float::INFINITY)
          next
        end

        came_from[neighbor] = current
        g_score[neighbor] = candidate_g
        f_score[neighbor] = g_score[neighbor] + @graph.heuristic(neighbor, goal)
      end
    end

    raise RuntimeError, "A* failed to find a solution"
  end
end
