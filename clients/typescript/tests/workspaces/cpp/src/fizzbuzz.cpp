#include "fizzbuzz.hpp"
#include <string>

std::string fizzbuzz(int n) {
  if (n % 15 == 0) return "fizzbuzz";
  if (n % 3 == 0)  return "fizz";
  if (n % 5 == 0)  return "buzz";
  return std::to_string(n);
}
