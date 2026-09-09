# Classroom materials and live screen sharing

[Pointory](../../README.md) · [Guide index](README.md) · [한국어](../KR/10-classroom-sharing.md)

![Instructor sharing settings in Pointory](../assets/sharing-settings.jpg)

*Settings is the actual UI in browser preview. The student screenshot uses the real local HTTP server with an example file and a static sample frame. Neither screenshot proves native desktop capture or connectivity from another classroom device.*

## Publish materials as an instructor

1. Open **Share materials** in Pointory settings.
2. Click **Add files** and choose PDFs, slides, documents, or exported PNGs. Only selected files are listed; no directory is published. Up to 100 files are supported and duplicate paths are skipped.
3. Check the detected IPv4 address belongs to the classroom Wi-Fi or Ethernet network. If it belongs to a VPN, enter the correct local IPv4 address.
4. Click **Start sharing**. The app selects an available port and displays a URL and QR code.
5. Copy the URL or show the QR. Students use a browser on the same network; no app or account is required.

Anyone with the URL can download. HTTP is unencrypted; use a trusted classroom network. Removing a file revokes subsequent access, but cannot recover copies already downloaded. Moving or deleting the selected original makes it unavailable. Editing the original changes what is downloaded.

## Show the current annotated screen

1. Select the annotation monitor and prepare its contents.
2. With sharing running, click **Start live view**.
3. Check the student page's live area.
4. Click **Stop live view** before unrelated work. Material downloads remain available.

The entire selected monitor is captured, including ink, toolbar, open windows, and notifications. Move the sharing panel to another monitor or close it if needed. Changing the selected monitor stops the broadcast; start it again deliberately. Frames are JPEG images up to 1280×720, refreshed at approximately 2 fps with no audio. This is for following annotations, not high-motion video or conferencing; actual speed depends on the PC and network.

![Actual student page serving a static example frame](../assets/sharing-student.jpg)

## Student steps

1. Join the instructor's network.
2. Scan the QR or open the full `http://IPv4:port/random-path/` URL. Keep the random path.
3. Click a material's filename to download directly from the instructor PC. Inline PDF preview is not implemented.
4. The live area updates when enabled by the instructor. Use **Refresh** to update the material list.

## Stop and troubleshoot

- **Stop sharing** or quitting Pointory stops the server and live view. Closing only the panel keeps sharing on.
- Restarting sharing rotates the random URL and may change the port. Send students the new address. The file list lasts only for the current app session.
- Allow Pointory through the Windows firewall on the required private network. The app does not change firewall rules or configure router port forwarding.
- Guest/school Wi-Fi client isolation, VLAN separation, VPNs, and the wrong IPv4 address can block access. Ask the network administrator whether device-to-device traffic is allowed.
- Private/link-local IPv4 networks are the target. Internet hosting, IPv6, HTTPS, uploads, and account management are not included.
- If live view stays unavailable, check the instructor panel's error, screen capture permissions, and selected monitor. Native macOS/Linux sharing, cross-device classroom connectivity, and classroom-scale load testing remain unverified.
