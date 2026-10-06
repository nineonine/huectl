// SPDX-License-Identifier: GPL-2.0 OR MIT
/*
 * huectl: HID driver for the huectl controller (Pico, or tools/fake-pico).
 *
 * Step 1: claim the device and log raw reports. Input mapping is still
 * the HID core's generic one (HID_CONNECT_DEFAULT).
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
};
module_hid_driver(huectl_driver);

MODULE_LICENSE("Dual MIT/GPL");
MODULE_DESCRIPTION("HID driver for the huectl Hue controller");
