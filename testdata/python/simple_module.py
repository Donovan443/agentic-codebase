"""A simple Python module for testing the parser."""

import os
from collections import OrderedDict
from typing import List, Optional


class Animal:
    """Base class for animals."""

    def __init__(self, name: str, age: int):
        """Initialize an animal."""
        self.name = name
        self.age = age

    def speak(self) -> str:
        """Make a sound."""
        return ""

    def _internal_method(self):
        """A private-by-convention method."""
        pass

    def __private_method(self):
        """A name-mangled private method."""
        pass


class Dog(Animal):
    """A dog that inherits from Animal."""

    def speak(self) -> str:
        return f"{self.name} says Woof!"


async def fetch_data(url: str) -> dict:
    """Fetch data from a URL asynchronously."""
    if url.startswith("http"):
        return {"status": "ok"}
    return {"status": "error"}


def process_items(items: List[str]) -> int:
    """Process a list of items with some complexity."""
    count = 0
    for item in items:
        if item.startswith("a"):
            count += 1
        elif item.startswith("b"):
            count += 2
        else:
            for char in item:
                if char.isdigit():
                    count += 10
    return count


def _private_helper():
    """A module-level private function."""
    pass


def __very_private():
    """A module-level very private function."""
    pass


def test_animals():
    """Test function for animals."""
    dog = Dog("Rex", 5)
    assert dog.speak() == "Rex says Woof!"


def generator_func():
    """A generator function."""
    for i in range(10):
        yield i * 2
