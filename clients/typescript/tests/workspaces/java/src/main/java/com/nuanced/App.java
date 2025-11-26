package com.nuanced;

import java.util.*;

public class App {
  public static void main(String[] args) {
    int total = Util.add(2, 3);
    Util.Greeter g = new Util.Greeter("Nuanced");
    List<String> series = new ArrayList<>();
    for (int i = 1; i <= 20; i++) series.add(FizzBuzz.fizzBuzz(i));
    System.out.println(total + " " + g.greet() + " " + series);
  }
}
