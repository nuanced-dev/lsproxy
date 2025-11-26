package com.nuanced;

public final class Util {
  public static int add(int a, int b) { return a + b; }

  public static class Greeter {
    private final String name;
    public Greeter(String name) { this.name = name; }
    public String greet() { return "Hello, " + name + "!"; }
  }
}
