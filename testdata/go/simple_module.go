// Package simple provides a simple Go module for testing the parser.
package simple

import (
	"fmt"
	"strings"
)

// Animal represents an animal.
type Animal struct {
	Name string
	Age  int
}

// Speak returns the animal's speech.
func (a *Animal) Speak() string {
	return fmt.Sprintf("%s speaks", a.Name)
}

// internal is an unexported type.
type internal struct {
	count int
}

// Speaker defines the interface for things that speak.
type Speaker interface {
	Speak() string
}

// ProcessItems processes a list of items.
func ProcessItems(items []string) int {
	count := 0
	for _, item := range items {
		if strings.HasPrefix(item, "a") {
			count++
		}
	}
	return count
}

// privateHelper is an unexported function.
func privateHelper() string {
	return "helper"
}

// NewAnimal creates a new animal.
func NewAnimal(name string, age int) *Animal {
	return &Animal{Name: name, Age: age}
}
