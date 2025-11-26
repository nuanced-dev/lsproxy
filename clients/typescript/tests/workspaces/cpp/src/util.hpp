#pragma once
#include <string>

int add(int a, int b);

struct Greeter {
  std::string name;
  explicit Greeter(std::string name) : name(std::move(name)) {}
  std::string greet() const { return "Hello, " + name + "!"; }
};
