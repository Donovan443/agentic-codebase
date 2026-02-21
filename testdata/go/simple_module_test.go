package simple

import "testing"

func TestNewAnimal(t *testing.T) {
	a := NewAnimal("Rex", 5)
	if a.Name != "Rex" {
		t.Errorf("expected Rex, got %s", a.Name)
	}
}

func TestProcessItems(t *testing.T) {
	items := []string{"apple", "banana", "avocado"}
	count := ProcessItems(items)
	if count != 2 {
		t.Errorf("expected 2, got %d", count)
	}
}

func BenchmarkProcessItems(b *testing.B) {
	items := []string{"apple", "banana", "avocado"}
	for i := 0; i < b.N; i++ {
		ProcessItems(items)
	}
}
