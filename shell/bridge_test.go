package main

import (
	"bufio"
	"encoding/json"
	"io"
	"strings"
	"sync"
	"testing"
	"time"
)

func TestBridgeRoutesConcurrentRepliesAndEOF(t *testing.T) {
	fromReader, fromWriter := io.Pipe()
	toReader, toWriter := io.Pipe()
	c := connect(fromReader, toWriter)
	go func() {
		s := bufio.NewScanner(toReader)
		for s.Scan() {
			var req map[string]json.RawMessage
			json.Unmarshal(s.Bytes(), &req)
			if string(req["method"]) == `"close"` {
				fromWriter.Close()
				return
			}
			b, _ := json.Marshal(map[string]any{"id": req["id"], "result": req["params"]})
			fromWriter.Write(append(b, '\n'))
		}
	}()
	var wg sync.WaitGroup
	for i := 0; i < 12; i++ {
		wg.Add(1)
		go func(n int) {
			defer wg.Done()
			p, _ := json.Marshal(n)
			result, err := c.call("echo", p)
			if err != nil || string(result) != string(p) {
				t.Errorf("reply %s, %v", result, err)
			}
		}(i)
	}
	wg.Wait()
	if _, err := c.call("close", nil); err == nil {
		t.Fatal("EOF must fail outstanding call")
	}
}
func TestPageIsLocalBoundedAndContainsNoRemovedNavigation(t *testing.T) {
	p := page(false)
	if len(p) >= 2*1024*1024 {
		t.Fatal("WebView2 NavigateToString size exceeded")
	}
	for _, s := range []string{"connect-src 'none'", "Diktator", "Einstellungen", "Verlauf"} {
		if !strings.Contains(p, s) {
			t.Fatalf("missing %s", s)
		}
	}
	for _, s := range []string{"AI Models", "Affiliate Program", "Power Mode", "data-nav=\"enhancement\""} {
		if strings.Contains(p, s) {
			t.Fatalf("removed feature remains: %s", s)
		}
	}
}

func TestBridgeTimeoutReleasesPendingAndAcceptsLateReply(t *testing.T) {
	fromReader, fromWriter := io.Pipe()
	toReader, toWriter := io.Pipe()
	defer fromWriter.Close()
	defer toReader.Close()
	c := connect(fromReader, toWriter)
	c.timeout = 30 * time.Millisecond
	go func() {
		scan := bufio.NewScanner(toReader)
		if !scan.Scan() {
			return
		}
		var req map[string]json.RawMessage
		json.Unmarshal(scan.Bytes(), &req)
		time.Sleep(100 * time.Millisecond)
		b, _ := json.Marshal(map[string]any{"id": req["id"], "result": true})
		fromWriter.Write(append(b, '\n'))
	}()
	start := time.Now()
	if _, err := c.call("state", nil); err == nil {
		t.Fatal("unresponsive core must time out")
	}
	if time.Since(start) > time.Second {
		t.Fatal("timeout did not bound UI request")
	}
	c.mu.Lock()
	pending := len(c.pending)
	c.mu.Unlock()
	if pending != 0 {
		t.Fatal("timeout leaked pending request")
	}
}

func TestBlockedPipeWriteCannotFreezeCalls(t *testing.T) {
	fromReader, fromWriter := io.Pipe()
	toReader, toWriter := io.Pipe()
	defer fromWriter.Close()
	defer toReader.Close()
	defer toWriter.Close()
	c := connect(fromReader, toWriter)
	c.timeout = 30 * time.Millisecond
	start := time.Now()
	if _, err := c.call("state", nil); err == nil {
		t.Fatal("blocked pipe must time out")
	}
	if time.Since(start) > time.Second {
		t.Fatal("blocked writer froze call")
	}
}
