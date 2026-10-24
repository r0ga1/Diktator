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
)

//go:embed ui/*
var files embed.FS
var version = "0.12.0"

type reply struct {
	ID     uint64          `json:"id"`
	Result json.RawMessage `json:"result"`
	Error  string          `json:"error"`
}
type client struct {
	w       io.Writer
	mu      sync.Mutex
	writeMu sync.Mutex
	pending map[uint64]chan reply
	next    atomic.Uint64
	closed  bool
}

func connect(r io.Reader, w io.Writer) *client {
	c := &client{w: w, pending: map[uint64]chan reply{}}
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
		c.mu.Lock()
		c.closed = true
		for id, ch := range c.pending {
			ch <- reply{Error: "Verbindung zum Diktierkern beendet"}
			delete(c.pending, id)
		}
		c.mu.Unlock()
	}()
	return c
}
func (c *client) call(method string, params json.RawMessage) (json.RawMessage, error) {
	id := c.next.Add(1)
	ch := make(chan reply, 1)
	c.mu.Lock()
	if c.closed {
		c.mu.Unlock()
		return nil, errors.New("Diktierkern ist nicht verbunden")
	}
	c.pending[id] = ch
	c.mu.Unlock()
	b, _ := json.Marshal(map[string]any{"id": id, "method": method, "params": params})
	c.writeMu.Lock()
	_, err := fmt.Fprintf(c.w, "%s\n", b)
	c.writeMu.Unlock()
	if err != nil {
		c.mu.Lock()
		delete(c.pending, id)
		c.mu.Unlock()
		return nil, err
	}
	v := <-ch
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
