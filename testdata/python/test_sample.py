"""Test file for parser test file detection."""

import pytest
import unittest


def test_addition():
    assert 1 + 1 == 2


def test_subtraction():
    assert 5 - 3 == 2


class TestMath(unittest.TestCase):
    def test_multiply(self):
        self.assertEqual(2 * 3, 6)
