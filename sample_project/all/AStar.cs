namespace AStarPathfinding
{
    public class AStar
    {
        private readonly List<Node> _open = new();
        private readonly List<Node> _closed = new();
        private readonly List<Node> _path = new();
        private readonly int[][] _maze;
        private Node _current;
        private readonly int _xStart;
        private readonly int _yStart;
        private int _xEnd, _yEnd;
        private readonly bool _diag;

        public AStar(int[][] maze, int xStart, int yStart, bool diag)
        {
            _maze = maze;
            _current = new Node(null, xStart, yStart, 0, 0);
            _xStart = xStart;
            _yStart = yStart;
            _diag = diag;
        }

        public List<Node>? FindPathTo(int xEnd, int yEnd)
        {
            _xEnd = xEnd;
            _yEnd = yEnd;
            _closed.Add(_current);
            AddNeighborsToOpenList();

            while (_current.X != _xEnd || _current.Y != _yEnd)
            {
                if (!_open.Any())
                    return null;

                _current = _open[0];
                _open.RemoveAt(0);
                _closed.Add(_current);
                AddNeighborsToOpenList();
            }

            _path.Insert(0, _current);
            while (_current.X != _xStart || _current.Y != _yStart)
            {
                _current = _current.Parent!;
                _path.Insert(0, _current);
            }

            return _path;
        }

        private void AddNeighborsToOpenList()
        {
            for (int x = -1; x <= 1; x++)
            {
                for (int y = -1; y <= 1; y++)
                {
                    if (!_diag && x != 0 && y != 0)
                        continue;

                    var node = new Node(_current, _current.X + x, _current.Y + y, _current.G, Distance(x, y));

                    if ((x != 0 || y != 0) &&
                        _current.X + x >= 0 && _current.X + x < _maze[0].Length &&
                        _current.Y + y >= 0 && _current.Y + y < _maze.Length &&
                        _maze[_current.Y + y][_current.X + x] != -1 &&
                        !FindNeighborInList(_open, node) &&
                        !FindNeighborInList(_closed, node))
                    {
                        node.G = node.Parent!.G + 1.0;
                        node.G += _maze[_current.Y + y][_current.X + x];
                        _open.Add(node);
                    }
                }
            }
            _open.Sort();
        }

        private double Distance(int x, int y)
        {
            return Math.Sqrt(Math.Pow(_xEnd - (_current.X + x), 2) + Math.Pow(_yEnd - (_current.Y + y), 2));
        }

        private bool FindNeighborInList(List<Node> list, Node node)
        {
            return list.Any(n => n.X == node.X && n.Y == node.Y);
        }
    }
}
