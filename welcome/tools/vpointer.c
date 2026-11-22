/* vpointer — a virtual pointer that stays connected, for the headless
 * screenshots (wlrctl's pointer goes away after each command, and with it the
 * seat's pointer capability, so clicks never reach the window).
 *
 * Reads commands from stdin, one per line:
 *   move <x> <y>      absolute position, in pixels of a <width>x<height> output
 *   click             left button, press and release
 *   scroll <amount>   vertical scroll (positive = down)
 *
 * Build (in the container, see headless-env.sh):
 *   wayland-scanner client-header  <xml> wlr-virtual-pointer.h
 *   wayland-scanner private-code   <xml> wlr-virtual-pointer.c
 *   cc -o vpointer vpointer.c wlr-virtual-pointer.c -lwayland-client
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>
#include <wayland-client.h>
#include "wlr-virtual-pointer.h"

static struct zwlr_virtual_pointer_manager_v1 *manager;

static void global(void *data, struct wl_registry *registry, uint32_t name, const char *interface, uint32_t version) {
    (void)data; (void)version;
    if (strcmp(interface, zwlr_virtual_pointer_manager_v1_interface.name) == 0)
        manager = wl_registry_bind(registry, name, &zwlr_virtual_pointer_manager_v1_interface, 1);
}
static void global_remove(void *data, struct wl_registry *registry, uint32_t name) { (void)data; (void)registry; (void)name; }
static const struct wl_registry_listener listener = { global, global_remove };

static uint32_t now(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint32_t)(ts.tv_sec * 1000 + ts.tv_nsec / 1000000);
}

int main(int argc, char **argv) {
    if (argc != 3) { fprintf(stderr, "usage: vpointer <width> <height>\n"); return 2; }
    uint32_t width = (uint32_t)atoi(argv[1]), height = (uint32_t)atoi(argv[2]);
    struct wl_display *display = wl_display_connect(NULL);
    if (!display) { fprintf(stderr, "vpointer: no Wayland display\n"); return 1; }
    struct wl_registry *registry = wl_display_get_registry(display);
    wl_registry_add_listener(registry, &listener, NULL);
    wl_display_roundtrip(display);
    if (!manager) { fprintf(stderr, "vpointer: the compositor has no virtual pointer protocol\n"); return 1; }
    struct zwlr_virtual_pointer_v1 *pointer = zwlr_virtual_pointer_manager_v1_create_virtual_pointer(manager, NULL);
    wl_display_roundtrip(display);

    char line[128];
    while (fgets(line, sizeof line, stdin)) {
        int x, y, amount;
        if (sscanf(line, "move %d %d", &x, &y) == 2) {
            zwlr_virtual_pointer_v1_motion_absolute(pointer, now(), (uint32_t)x, (uint32_t)y, width, height);
            zwlr_virtual_pointer_v1_frame(pointer);
        } else if (strncmp(line, "click", 5) == 0) {
            zwlr_virtual_pointer_v1_button(pointer, now(), 0x110, WL_POINTER_BUTTON_STATE_PRESSED);
            zwlr_virtual_pointer_v1_frame(pointer);
            wl_display_roundtrip(display);
            usleep(80000);
            zwlr_virtual_pointer_v1_button(pointer, now(), 0x110, WL_POINTER_BUTTON_STATE_RELEASED);
            zwlr_virtual_pointer_v1_frame(pointer);
        } else if (sscanf(line, "scroll %d", &amount) == 1) {
            zwlr_virtual_pointer_v1_axis(pointer, now(), WL_POINTER_AXIS_VERTICAL_SCROLL, wl_fixed_from_int(amount));
            zwlr_virtual_pointer_v1_frame(pointer);
        }
        wl_display_roundtrip(display);
    }
    return 0;
}
