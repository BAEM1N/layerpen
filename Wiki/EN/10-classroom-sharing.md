# Classroom materials and live screen sharing

[Pointory](../../README.md) · [Guide index](README.md) · [한국어](../KR/10-classroom-sharing.md)

![Instructor sharing settings in Pointory](../assets/sharing-settings.jpg)

*Settings is the actual UI in browser preview. The student screenshot uses the real local HTTP server with an example file and a static sample frame. Neither screenshot proves native desktop capture or connectivity from another classroom device.*

## Publish materials as an instructor

1. Open **Share materials** in Pointory settings.
2. Click **Add files** and choose PDFs, slides, documents, or exported PNGs. Only selected files are listed; no directory is published. Up to 100 files are supported and duplicate paths are skipped.
3. Choose an IPv4 address that students can reach on the instructor PC's sharing port and that school or organization policy permits. Use the Wi-Fi, Ethernet, or VPN address matching that route. A VPN is usable when device-to-device TCP access and policy requirements are satisfied.
4. Click **Start sharing**. The app selects an available port and displays a URL and QR code.
5. Copy the URL or show the QR. Students use a browser on a classroom network that can reach the instructor PC; no app or account is required.

Anyone with the URL can download. HTTP is unencrypted; use a trusted classroom network. Removing a file revokes subsequent access, but cannot recover copies already downloaded. Moving or deleting the selected original makes it unavailable. Editing the original changes what is downloaded.

## Show the current annotated screen

1. Select the annotation monitor and prepare its contents.
2. With sharing running, click **Start live view**.
3. Check the student page's live area.
4. Click **Stop live view** before unrelated work. Material downloads remain available.

The entire selected monitor is captured, including ink, toolbar, open windows, and notifications. Move the sharing panel to another monitor or close it if needed. Changing the selected monitor stops the broadcast; start it again deliberately. Frames are JPEG images up to 1280×720, refreshed at approximately 2 fps with no audio. This is for following annotations, not high-motion video or conferencing; actual speed depends on the PC and network.

![Actual student page serving a static example frame](../assets/sharing-student.jpg)

## Student steps

1. Join a school-approved classroom network or VPN that can reach the instructor PC.
2. Scan the QR or open the full `http://IPv4:port/random-path/` URL. Keep the random path.
3. Click a material's filename to download directly from the instructor PC. Inline PDF preview is not implemented.
4. The live area updates when enabled by the instructor. Use **Refresh** to update the material list.

## Stop and troubleshoot

- **Stop sharing** or quitting Pointory stops the server and live view. Closing only the panel keeps sharing on.
- Restarting sharing rotates the random URL and may change the port. Send students the new address. The file list lasts only for the current app session.
- Allow Pointory through the Windows firewall on the required private network. The app does not change firewall rules or configure router port forwarding.
- Guest/school Wi-Fi client isolation, routing or policies between VLANs, VPN restrictions on device-to-device traffic, and the wrong IPv4 address can block access. Ask the network administrator whether TCP connections from **student devices to the instructor PC's sharing port** are allowed. A successful SSH connection from the instructor PC to another device does not establish HTTP connectivity in the reverse direction.
- Private/link-local IPv4 networks are the target. Internet hosting, IPv6, HTTPS, uploads, and account management are not included.
- If live view stays unavailable, check the instructor panel's error, screen capture permissions, and selected monitor. Native macOS/Linux desktop sharing, direct classroom connectivity, and classroom-scale load testing remain unverified.

## Verified connection scope

On 2026-09-11, a Mac headless browser downloaded files and displayed changing
synthetic frames from the real Windows sharing server through a temporary SSH
forwarding route. Direct HTTP requests from the Mac to the Windows VPN address
timed out; the cause remains undetermined. This did not test actual desktop
capture, a logged-in Mac GUI, or successful direct classroom connectivity.
See the [validation results and limits (Korean)](../../docs/validation/0.2-windows-share-mac.ko.md).
