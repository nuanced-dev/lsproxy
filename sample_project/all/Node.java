package sample_project.java;

public class Node implements Comparable<Node> {
    public Node parent;
    public int x, y;
    public double g;
    public double h;

    Node(Node parent, int xpos, int ypos, double g, double h) {
        this.parent = parent;
        this.x = xpos;
        this.y = ypos;
        this.g = g;
        this.h = h;
    }

    @Override
    public int compareTo(Node o) {
        return Double.compare(this.g + this.h, o.g + o.h);
    }
} 