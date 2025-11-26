#include "fizzbuzz.hpp"
#include "util.hpp"
#include <iostream>
#include <vector>

int main() {
  int total = add(2, 3);
  Greeter g{"Nuanced"};
  std::vector<std::string> series;
  series.reserve(20);
  for (int i = 1; i <= 20; ++i) series.push_back(fizzbuzz(i));

  std::cout << total << " " << g.greet() << " [";
  for (size_t i = 0; i < series.size(); ++i) {
    if (i) std::cout << ", ";
    std::cout << series[i];
  }
  std::cout << "]\n";
  return 0;
}
