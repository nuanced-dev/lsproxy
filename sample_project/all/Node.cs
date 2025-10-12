namespace AStarPathfinding
{
    public class Node : IComparable<Node>
    {
        public Node? Parent { get; set; }
        public int X { get; set; }
        public int Y { get; set; }
        public double G { get; set; }
        public double H { get; set; }

        public Node(Node? parent, int x, int y, double g, double h)
        {
            Parent = parent;
            X = x;
            Y = y;
            G = g;
            H = h;
        }

        public int CompareTo(Node? other)
        {
            if (other == null) return 1;
            return (G + H).CompareTo(other.G + other.H);
        }
    }
}
