// SPDX-License-Identifier: GPL-2.0 OR MIT
/*
 * huectl: HID driver for the huectl controller (Pico, or tools/fake-pico).
 *
 * Claims the device, logs raw reports, and maps each control to a fixed
 * input event code (huectl_input_mapping). The HID core still does the
 * report parsing and event generation; we only choose the codes.
 *
 * Thin by design (CLAUDE.md): parse reports, expose standard interfaces.
 * No Hue knowledge here.
 */
#include <linux/hid.h>
#include <linux/module.h>

#define HUECTL_VID	0x1209	/* pid.codes */
#define HUECTL_PID	0x0001	/* pid.codes test PID, for development */

/*
 * Called for every report from the device, before the HID core's own
 * processing. Runs in interrupt context on real USB: no sleeping here.
 * Returning 0 lets the core carry on and generate the input events.
 */
static int huectl_raw_event(struct hid_device *hdev, struct hid_report *report,
			    u8 *data, int size)
{
	/* Rate-limited: a busy encoder must not flood the kernel log. */
	dev_info_ratelimited(&hdev->dev, "report %u (%d bytes): %*ph\n",
			     report->id, size, size, data);
	return 0;
}

/*
 * The event codes below are the driver's contract with the daemon.
 *
 *   Button 1 (encoder push)  -> EV_KEY BTN_0
 *   Buttons 2-4              -> EV_KEY BTN_1..BTN_3
 *   Dial (encoder, relative) -> EV_REL REL_DIAL
 *   Slider (pot, 0..1023)    -> EV_ABS ABS_MISC
 *
 * BTN_* rather than KEY_*: a desktop would treat KEY_* as a keyboard and
 * could act on the presses.
 */
static const unsigned int huectl_buttons[] = { BTN_0, BTN_1, BTN_2, BTN_3 };

/*
 * Called once per usage in the descriptor while the input device is being
 * built. Return 1 = we mapped it, -1 = ignore it, 0 = use hid-input's
 * default guess. We never return 0, so nothing unplanned leaks through.
 * For EV_ABS the core still sets min/max from the descriptor.
 */
static int huectl_input_mapping(struct hid_device *hdev, struct hid_input *hi,
				struct hid_field *field, struct hid_usage *usage,
				unsigned long **bit, int *max)
{
	unsigned int page = usage->hid & HID_USAGE_PAGE;
	unsigned int id = usage->hid & HID_USAGE;

	switch (usage->hid) {
	case HID_GD_DIAL:
		hid_map_usage(hi, usage, bit, max, EV_REL, REL_DIAL);
		return 1;
	case HID_GD_SLIDER:
		hid_map_usage(hi, usage, bit, max, EV_ABS, ABS_MISC);
		return 1;
	}

	if (page == HID_UP_BUTTON && id >= 1 && id <= ARRAY_SIZE(huectl_buttons)) {
		hid_map_usage(hi, usage, bit, max, EV_KEY, huectl_buttons[id - 1]);
		return 1;
	}

	/* E.g. the vendor-defined LED usage: not an input event. */
	return -1;
}

static int huectl_probe(struct hid_device *hdev, const struct hid_device_id *id)
{
	int ret;

	/* Parse the report descriptor into hdev->report_enum[]. */
	ret = hid_parse(hdev);
	if (ret) {
		hid_err(hdev, "report descriptor parse failed: %d\n", ret);
		return ret;
	}

	/* Start the transport and connect the generic input/hidraw layers. */
	ret = hid_hw_start(hdev, HID_CONNECT_DEFAULT);
	if (ret) {
		hid_err(hdev, "hw start failed: %d\n", ret);
		return ret;
	}

	hid_info(hdev, "huectl controller bound\n");
	return 0;
}

static void huectl_remove(struct hid_device *hdev)
{
	hid_hw_stop(hdev);
	hid_info(hdev, "huectl controller removed\n");
}

/* Must match VID/PID, or hid-generic binds instead (CLAUDE.md decision 7). */
static const struct hid_device_id huectl_devices[] = {
	{ HID_USB_DEVICE(HUECTL_VID, HUECTL_PID) },
	{ }
};
/* Exported so udev can autoload this module when the device appears. */
MODULE_DEVICE_TABLE(hid, huectl_devices);

static struct hid_driver huectl_driver = {
	.name		= "huectl",
	.id_table	= huectl_devices,
	.probe		= huectl_probe,
	.remove		= huectl_remove,
	.raw_event	= huectl_raw_event,
	.input_mapping	= huectl_input_mapping,
};
module_hid_driver(huectl_driver);

MODULE_LICENSE("Dual MIT/GPL");
MODULE_DESCRIPTION("HID driver for the huectl Hue controller");
