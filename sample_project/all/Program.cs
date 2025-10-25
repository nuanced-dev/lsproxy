namespace AStarPathfinding
{
    class Program
    {
        static void Main(string[] args)
        {
            // -1 = blocked
            // 0+ = additional movement cost
            int[][] maze = new int[][]
            {
                new int[] { 0, 0, 0, 0, 0, 0, 0, 0 },
                new int[] { 0, 0, 0, 0, 0, 0, 0, 0 },
                new int[] { 0, 0, 0, 100, 100, 100, 0, 0 },
                new int[] { 0, 0, 0, 0, 0, 100, 0, 0 },
                new int[] { 0, 0, 100, 0, 0, 100, 0, 0 },
                new int[] { 0, 0, 100, 0, 0, 100, 0, 0 },
                new int[] { 0, 0, 100, 100, 100, 100, 0, 0 },
                new int[] { 0, 0, 0, 0, 0, 0, 0, 0 }
            };

            var aStar = new AStar(maze, 0, 0, true);
            var path = aStar.FindPathTo(7, 7);

            if (path != null)
            {
                foreach (var node in path)
                {
                    Console.Write($"[{node.X}, {node.Y}] ");
                    maze[node.Y][node.X] = -1;
                }
                Console.WriteLine($"\nTotal cost: {path[^1].G:F2}");

                foreach (var row in maze)
                {
                    foreach (var entry in row)
                    {
                        Console.Write(entry switch
                        {
                            0 => "_",
                            -1 => "*",
                            _ => "#"
                        });
                    }
                    Console.WriteLine();
                }
            }
        }
    }
}
