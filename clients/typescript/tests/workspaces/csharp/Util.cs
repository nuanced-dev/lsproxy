static class Util {
    public static int Add(int a, int b) => a + b;

    public sealed class Greeter(string name) {
        public string Greet() => $"Hello, {name}!";
    }
}
