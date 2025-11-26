from app.fizzbuzz import fizz_buzz
from app.util import greet


def main(n: int) -> list[str]:
    return [fizz_buzz(i) for i in range(1, n + 1)]


if __name__ == "__main__":
    print(", ".join(main(15)))
    print(greet("world"))
