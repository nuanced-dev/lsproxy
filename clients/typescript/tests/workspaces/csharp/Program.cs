uusing System;
using System.Linq;

class Program {
    static void Main() {
        var total = Util.Add(2, 3);
        var g = new Util.Greeter("Nuanced");
        var series = Enumerable.Range(1, 20).Select(FizzBuzz.Of).ToArray();
        Console.WriteLine($"{total} {g.Greet()} [{string.Join(", ", series)}]");
    }
}sing System;

class Program {
    static string FizzBuzz(int n) {
        if (n % 15 == 0) return "fizzbuzz";
        if (n % 3 == 0) return "fizz";
        if (n % 5 == 0) return "buzz";
        return n.ToString();
    }

    static string Repeat(string s, int times) {
        return new String(s[0], times) + s.Substring(1); // simple repeat
    }

    static void Main(string[] args) {
        for (int i = 1; i <= 15; i++) {
            Console.WriteLine(FizzBuzz(i));
        }
    }
}
