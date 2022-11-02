package main

import (
	"bufio"
	"fmt"
	"os"
	"time"
)

func main() {
	reader := bufio.NewReader(os.Stdin)
	writer := bufio.NewWriter(os.Stdout)
	reader.ReadString('\n')
	time.Sleep(4 * time.Second)
	fmt.Fprintf(writer, "str abcdefghijk\n")
	writer.Flush()
    time.Sleep(4 * time.Second)
    fmt.Fprintf(writer, "str ABCDEFGHIJK\n")
    writer.Flush()
    time.Sleep(4 * time.Second)
	fmt.Fprintf(writer, "end\n")
	writer.Flush()
}
