package main

import (
	"bufio"
	"embed"
	"encoding/base64"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"sync/atomic"
	"time"
)

//go:embed ui/*
var files embed.FS
var version = "0.13.0"

type reply struct {
	ID     uint64          `json:"id"`
	Result json.RawMessage `json:"result"`
	Error  string          `json:"error"`
}
type request struct {
	id   uint64
	data []byte
}
type client struct {
	mu      sync.Mutex
	pending map[uint64]chan reply
	writes  chan request
	done    chan struct{}
	next    atomic.Uint64
	closed  bool
	timeout time.Duration
}

func connect(r io.Reader, w io.Writer) *client {
	c := &client{pending: map[uint64]chan reply{}, writes: make(chan request, 32), done: make(chan struct{}), timeout: 20 * time.Second}
	go func() {
		for {
			select {
			case packet := <-c.writes:
				c.mu.Lock()
				_, active := c.pending[packet.id]
				c.mu.Unlock()
				if !active {
					continue
				}
				if _, err := w.Write(packet.data); err != nil {
					c.fail("Verbindung zum Diktierkern beendet")
					return
				}
			case <-c.done:
				return
			}
		}
	}()
	go func() {
		scan := bufio.NewScanner(r)
		scan.Buffer(make([]byte, 4096), 8*1024*1024)
		for scan.Scan() {
			var v reply
			if json.Unmarshal(scan.Bytes(), &v) != nil {
				continue
			}
			c.mu.Lock()
			ch := c.pending[v.ID]
			delete(c.pending, v.ID)
			c.mu.Unlock()
			if ch != nil {
				ch <- v
			}
		}
		c.fail("Verbindung zum Diktierkern beendet")
	}()
	return c
}
func (c *client) fail(message string) {
	c.mu.Lock()
	defer c.mu.Unlock()
	if c.closed {
		return
	}
	c.closed = true
	close(c.done)
	for id, ch := range c.pending {
		ch <- reply{Error: message}
		delete(c.pending, id)
	}
}
func (c *client) call(method string, params json.RawMessage) (json.RawMessage, error) {
	id := c.next.Add(1)
	ch := make(chan reply, 1)
	c.mu.Lock()
	if c.closed {
		c.mu.Unlock()
		return nil, errors.New("Diktierkern ist nicht verbunden")
	}
	if len(c.pending) >= 32 {
		c.mu.Unlock()
		return nil, errors.New("Diktierkern ist beschäftigt; bitte kurz warten")
	}
	c.pending[id] = ch
	c.mu.Unlock()
	defer func() { c.mu.Lock(); delete(c.pending, id); c.mu.Unlock() }()
	b, _ := json.Marshal(map[string]any{"id": id, "method": method, "params": params})
	if len(b) >= 8*1024*1024 {
		return nil, errors.New("Anfrage ist zu groß")
	}
	deadline := c.timeout
	if method == "import" || method == "exportEntry" {
		deadline = 10 * time.Minute
	}
	timer := time.NewTimer(deadline)
	defer timer.Stop()
	timeout := func() error {
		return errors.New("Der Diktierkern antwortet nicht rechtzeitig. Die Aktion konnte nicht bestätigt werden; das Fenster bleibt bedienbar")
	}
	select {
	case c.writes <- request{id, append(b, '\n')}:
	case <-c.done:
		return nil, errors.New("Diktierkern ist nicht verbunden")
	case <-timer.C:
		return nil, timeout()
	}
	var v reply
	select {
	case v = <-ch:
	case <-c.done:
		return nil, errors.New("Diktierkern ist nicht verbunden")
	case <-timer.C:
		return nil, timeout()
	}
	if v.Error != "" {
		return nil, errors.New(v.Error)
	}
	return v.Result, nil
}

func page(preview bool) string {
	readAsset := files.ReadFile
	if preview {
		if root := os.Getenv("DIKTATOR_PREVIEW_ASSETS"); root != "" {
			readAsset = func(name string) ([]byte, error) { return os.ReadFile(filepath.Join(root, name)) }
		}
	}
	html, _ := readAsset("ui/index.html")
	css, _ := readAsset("ui/style.css")
	js, _ := readAsset("ui/app.js")
	font, _ := readAsset("ui/rounded.ttf")
	logo, _ := readAsset("ui/logo.png")
	s := string(html)
	s = strings.ReplaceAll(s, "/* INLINE_CSS */", string(css))
	s = strings.ReplaceAll(s, "FONT_DATA", base64.StdEncoding.EncodeToString(font))
	s = strings.ReplaceAll(s, "LOGO_DATA", base64.StdEncoding.EncodeToString(logo))
	s = strings.ReplaceAll(s, "/* INLINE_JS */", string(js))
	bridgeBytes, _ := files.ReadFile("ui/native-bridge.js")
	bridge := string(bridgeBytes)
	if preview {
		bridge = `window.nativeCall=async(method,params)=>{const r=await fetch('/api',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({method,params:JSON.parse(params)})});const v=await r.json();if(v.error)throw Error(v.error);return v.result};`
		s = strings.Replace(s, "connect-src 'none'", "connect-src 'self'", 1)
	}
	s = strings.ReplaceAll(s, "/* BRIDGE */", bridge)
	return s
}
func report(e error) { fmt.Fprintln(os.Stderr, e) }
