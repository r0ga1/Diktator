//go:build !windows

package main

import (
	"context"
	"encoding/json"
	"flag"
	"fmt"
	"net/http"
	"os"
	"os/exec"
	"strings"
	"time"
)

func main() {
	address := flag.String("listen", "127.0.0.1:6086", "loopback preview address")
	command := flag.String("engine-command", "", "JSON command list for the real engine --stdio")
	flag.Parse()
	var args []string
	if json.Unmarshal([]byte(*command), &args) != nil || len(args) == 0 {
		fmt.Fprintln(os.Stderr, "Provide --engine-command with the real Windows worker, e.g. docker exec -i ... wine ... --stdio")
		os.Exit(1)
	}
	cmd := exec.Command(args[0], args[1:]...)
	out, _ := cmd.StdoutPipe()
	in, _ := cmd.StdinPipe()
	cmd.Stderr = os.Stderr
	if e := cmd.Start(); e != nil {
		panic(e)
	}
	defer cmd.Process.Kill()
	c := connect(out, in)
	http.HandleFunc("/api", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "POST" {
			http.Error(w, "POST required", 405)
			return
		}
		if o := r.Header.Get("Origin"); o != "" && o != "http://"+r.Host {
			http.Error(w, "Origin blocked", 403)
			return
		}
		var q struct {
			Method string          `json:"method"`
			Params json.RawMessage `json:"params"`
		}
		w.Header().Set("Content-Type", "application/json")
		if e := json.NewDecoder(http.MaxBytesReader(w, r.Body, 8*1024*1024)).Decode(&q); e != nil {
			json.NewEncoder(w).Encode(map[string]string{"error": e.Error()})
			return
		}
		v, e := c.call(q.Method, q.Params)
		if e != nil {
			json.NewEncoder(w).Encode(map[string]string{"error": e.Error()})
		} else {
			json.NewEncoder(w).Encode(map[string]any{"result": v})
		}
	})
	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/" {
			http.NotFound(w, r)
			return
		}
		w.Header().Set("Content-Type", "text/html; charset=utf-8")
		w.Header().Set("Cache-Control", "no-store")
		fmt.Fprint(w, page(true))
	})
	if !strings.HasPrefix(*address, "127.0.0.1:") {
		panic("Preview must bind loopback")
	}
	server := &http.Server{Addr: *address, ReadHeaderTimeout: 5 * time.Second}
	done := make(chan struct{})
	go func() {
		_ = cmd.Wait()
		ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
		defer cancel()
		_ = server.Shutdown(ctx)
		close(done)
	}()
	fmt.Println("Preview http://" + *address)
	if e := server.ListenAndServe(); e != nil && e != http.ErrServerClosed {
		panic(e)
	}
	<-done
}
