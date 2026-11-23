package main

import "testing"

func TestRunAllChecksCount(t *testing.T) {
	results := runAllChecks()
	if len(results) != 12 {
		t.Fatalf("got %d checks, want 12", len(results))
	}
	for _, r := range results {
		if r.ID == "" || r.Status == "" || r.Summary == "" {
			t.Fatalf("incomplete check item: %+v", r)
		}
	}
}
