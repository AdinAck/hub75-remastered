# hub75-remastered

A completely rewritten driver for HUB75 displays.

# Usage

The `embedded-hal` version must be selected with the feature gates `hal-02` or `hal-1`.

---

Create an instance of a display (for example 64x32)

```rust
type Display = Hub75_64_32_2<
    3, // color bits
    (/* upper color pins */),
    (/* lower color pins */),
    (/* row pins */),
    (/* data pins */),
>;

let mut display = Display::new(/* pins */);
```

---

In a continually running background task, draw to the display

```rust
async fn bg_task(display: Display) {
    loop {
        display.output(/* delay provider */);
        // maybe yield to other same priority tasks
    }
}
```
