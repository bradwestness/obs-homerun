package main

import (
	"context"
	"crypto/md5"
	"fmt"
	"io"
	"log"
	"net"
	"net/http"
	"os"
	"os/exec"
	"os/signal"
	"strconv"
	"strings"
	"syscall"
	"time"
)

var (
	hostIP        string
	httpPort      int
	friendlyName  string
	channelNumber string
	rtspSource    string
	bufferSec     float64
	halfBuffer    string
	audioCodec    string
	audioBitrate  string
	deviceUUID    string
)

func getDefaultHostIP() string {
	if env := os.Getenv("HOST_IP"); strings.TrimSpace(env) != "" {
		return strings.TrimSpace(env)
	}
	conn, err := net.Dial("udp", "8.8.8.8:80")
	if err == nil {
		defer conn.Close()
		localAddr := conn.LocalAddr().(*net.UDPAddr)
		return localAddr.IP.String()
	}
	return "127.0.0.1"
}

func generateUUID(name string) string {
	h := md5.Sum([]byte("obs-homerun." + name))
	h[6] = (h[6] & 0x0f) | 0x30 // version 3
	h[8] = (h[8] & 0x3f) | 0x80 // variant
	return fmt.Sprintf("%08x-%04x-%04x-%04x-%012x",
		h[0:4], h[4:6], h[6:8], h[8:10], h[10:16])
}

func initConfig() {
	hostIP = getDefaultHostIP()

	httpPort = 5004
	if env := os.Getenv("HTTP_PORT"); env != "" {
		if p, err := strconv.Atoi(env); err == nil {
			httpPort = p
		}
	}

	friendlyName = "OBS HomeRun"
	if env := os.Getenv("FRIENDLY_NAME"); env != "" {
		friendlyName = env
	}

	channelNumber = "1.1"
	if env := os.Getenv("CHANNEL_NUMBER"); env != "" {
		channelNumber = env
	}

	rtspSource = "rtsp://127.0.0.1:8554/live/stream"
	if env := os.Getenv("RTSP_SOURCE"); env != "" {
		rtspSource = env
	}

	bufferSec = 3.0
	if env := os.Getenv("BUFFER_SECONDS"); env != "" {
		if b, err := strconv.ParseFloat(env, 64); err == nil {
			bufferSec = b
		}
	}
	half := bufferSec / 2.0
	if half < 0.5 {
		half = 0.5
	}
	halfBuffer = fmt.Sprintf("%.1f", half)

	audioCodec = "ac3"
	if env := os.Getenv("AUDIO_CODEC"); env != "" {
		audioCodec = env
	}

	audioBitrate = "384k"
	if env := os.Getenv("AUDIO_BITRATE"); env != "" {
		audioBitrate = env
	}

	deviceUUID = generateUUID(friendlyName)
	if env := os.Getenv("DEVICE_UUID"); env != "" {
		deviceUUID = env
	}
}

func getDeviceXML() string {
	return fmt.Sprintf(`<?xml version="1.0" encoding="utf-8"?>
<root xmlns="urn:schemas-upnp-org:device-1-0" xmlns:dlna="urn:schemas-dlna-org:device-1-0">
  <specVersion>
    <major>1</major>
    <minor>0</minor>
  </specVersion>
  <device>
    <dlna:X_DLNADOC>DMS-1.50</dlna:X_DLNADOC>
    <deviceType>urn:schemas-upnp-org:device:MediaServer:1</deviceType>
    <friendlyName>%s</friendlyName>
    <presentationURL>/</presentationURL>
    <manufacturer>OBS HomeRun</manufacturer>
    <manufacturerURL>https://github.com/bradwestness/obs-homerun</manufacturerURL>
    <modelDescription>%s Virtual HDTV Tuner</modelDescription>
    <modelName>OBS HomeRun</modelName>
    <modelNumber>HDTV-1.0</modelNumber>
    <modelURL>https://github.com/bradwestness/obs-homerun</modelURL>
    <serialNumber>107BDESK</serialNumber>
    <UDN>uuid:%s</UDN>
    <serviceList>
      <service>
        <serviceType>urn:schemas-upnp-org:service:ConnectionManager:1</serviceType>
        <serviceId>urn:upnp-org:serviceId:ConnectionManager</serviceId>
        <SCPDURL>/dms/ConnectionManager.xml</SCPDURL>
        <controlURL>http://%s:%d/dms/ConnectionManager</controlURL>
        <eventSubURL>http://%s:%d/dms/ConnectionManager</eventSubURL>
      </service>
      <service>
        <serviceType>urn:schemas-upnp-org:service:ContentDirectory:1</serviceType>
        <serviceId>urn:upnp-org:serviceId:ContentDirectory</serviceId>
        <SCPDURL>/dms/ContentDirectory.xml</SCPDURL>
        <controlURL>http://%s:%d/dms/ContentDirectory</controlURL>
        <eventSubURL>http://%s:%d/dms/ContentDirectory</eventSubURL>
      </service>
    </serviceList>
  </device>
</root>`, friendlyName, friendlyName, deviceUUID, hostIP, httpPort, hostIP, httpPort, hostIP, httpPort, hostIP, httpPort)
}

const connectionManagerXML = `<?xml version="1.0" encoding="utf-8"?>
<scpd xmlns="urn:schemas-upnp-org:service-1-0">
  <specVersion><major>1</major><minor>0</minor></specVersion>
  <actionList>
    <action>
      <name>GetProtocolInfo</name>
      <argumentList>
        <argument><name>Source</name><direction>out</direction><relatedStateVariable>SourceProtocolInfo</relatedStateVariable></argument>
        <argument><name>Sink</name><direction>out</direction><relatedStateVariable>SinkProtocolInfo</relatedStateVariable></argument>
      </argumentList>
    </action>
    <action>
      <name>GetCurrentConnectionIDs</name>
      <argumentList>
        <argument><name>ConnectionIDs</name><direction>out</direction><relatedStateVariable>CurrentConnectionIDs</relatedStateVariable></argument>
      </argumentList>
    </action>
  </actionList>
  <serviceStateTable>
    <stateVariable sendEvents="yes"><name>SourceProtocolInfo</name><dataType>string</dataType></stateVariable>
    <stateVariable sendEvents="yes"><name>SinkProtocolInfo</name><dataType>string</dataType></stateVariable>
    <stateVariable sendEvents="yes"><name>CurrentConnectionIDs</name><dataType>string</dataType></stateVariable>
  </serviceStateTable>
</scpd>`

const contentDirectoryXML = `<?xml version="1.0" encoding="utf-8"?>
<scpd xmlns="urn:schemas-upnp-org:service-1-0">
  <specVersion><major>1</major><minor>0</minor></specVersion>
  <actionList>
    <action>
      <name>Browse</name>
      <argumentList>
        <argument><name>ObjectID</name><direction>in</direction><relatedStateVariable>A_ARG_TYPE_ObjectID</relatedStateVariable></argument>
        <argument><name>BrowseFlag</name><direction>in</direction><relatedStateVariable>A_ARG_TYPE_BrowseFlag</relatedStateVariable></argument>
        <argument><name>Filter</name><direction>in</direction><relatedStateVariable>A_ARG_TYPE_Filter</relatedStateVariable></argument>
        <argument><name>StartingIndex</name><direction>in</direction><relatedStateVariable>A_ARG_TYPE_Index</relatedStateVariable></argument>
        <argument><name>RequestedCount</name><direction>in</direction><relatedStateVariable>A_ARG_TYPE_Count</relatedStateVariable></argument>
        <argument><name>SortCriteria</name><direction>in</direction><relatedStateVariable>A_ARG_TYPE_SortCriteria</relatedStateVariable></argument>
        <argument><name>Result</name><direction>out</direction><relatedStateVariable>A_ARG_TYPE_Result</relatedStateVariable></argument>
        <argument><name>NumberReturned</name><direction>out</direction><relatedStateVariable>A_ARG_TYPE_Count</relatedStateVariable></argument>
        <argument><name>TotalMatches</name><direction>out</direction><relatedStateVariable>A_ARG_TYPE_Count</relatedStateVariable></argument>
        <argument><name>UpdateID</name><direction>out</direction><relatedStateVariable>A_ARG_TYPE_UpdateID</relatedStateVariable></argument>
      </argumentList>
    </action>
    <action>
      <name>GetSearchCapabilities</name>
      <argumentList><argument><name>SearchCaps</name><direction>out</direction><relatedStateVariable>SearchCapabilities</relatedStateVariable></argument></argumentList>
    </action>
    <action>
      <name>GetSortCapabilities</name>
      <argumentList><argument><name>SortCaps</name><direction>out</direction><relatedStateVariable>SortCapabilities</relatedStateVariable></argument></argumentList>
    </action>
    <action>
      <name>GetSystemUpdateID</name>
      <argumentList><argument><name>Id</name><direction>out</direction><relatedStateVariable>SystemUpdateID</relatedStateVariable></argument></argumentList>
    </action>
  </actionList>
  <serviceStateTable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_SortCriteria</name><dataType>string</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_UpdateID</name><dataType>ui4</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_Filter</name><dataType>string</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_Result</name><dataType>string</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_Index</name><dataType>ui4</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_ObjectID</name><dataType>string</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>SortCapabilities</name><dataType>string</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>SearchCapabilities</name><dataType>string</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_Count</name><dataType>ui4</dataType></stateVariable>
    <stateVariable sendEvents="no">
      <name>A_ARG_TYPE_BrowseFlag</name>
      <dataType>string</dataType>
      <allowedValueList><allowedValue>BrowseMetadata</allowedValue><allowedValue>BrowseDirectChildren</allowedValue></allowedValueList>
    </stateVariable>
    <stateVariable sendEvents="yes"><name>SystemUpdateID</name><dataType>ui4</dataType></stateVariable>
  </serviceStateTable>
</scpd>`

func getDIDLItem() string {
	return fmt.Sprintf(`&lt;DIDL-Lite xmlns="urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:upnp="urn:schemas-upnp-org:metadata-1-0/upnp/" xmlns:dlna="urn:schemas-dlna-org:metadata-1-0/"&gt;&lt;item id="v1" parentID="0" restricted="1"&gt;&lt;dc:title&gt;%s&lt;/dc:title&gt;&lt;upnp:class&gt;object.item.videoItem.videoBroadcast&lt;/upnp:class&gt;&lt;upnp:channelNr&gt;%s&lt;/upnp:channelNr&gt;&lt;upnp:channelName&gt;%s&lt;/upnp:channelName&gt;&lt;res protocolInfo="http-get:*:video/mpeg:DLNA.ORG_PN=AVC_TS_HD_60_AC3_ISO;DLNA.ORG_OP=00;DLNA.ORG_FLAGS=01700000000000000000000000000000"&gt;http://%s:%d/auto/v%s&lt;/res&gt;&lt;res protocolInfo="http-get:*:video/mp2t:*"&gt;http://%s:%d/auto/v%s&lt;/res&gt;&lt;res protocolInfo="http-get:*:video/vnd.dlna.mpeg-tts:DLNA.ORG_PN=AVC_TS_NA_ISO;DLNA.ORG_OP=00;DLNA.ORG_FLAGS=01700000000000000000000000000000"&gt;http://%s:%d/auto/v%s&lt;/res&gt;&lt;/item&gt;&lt;/DIDL-Lite&gt;`,
		friendlyName, channelNumber, friendlyName, hostIP, httpPort, channelNumber, hostIP, httpPort, channelNumber, hostIP, httpPort, channelNumber)
}

func getDIDLContainers() string {
	return fmt.Sprintf(`&lt;DIDL-Lite xmlns="urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:upnp="urn:schemas-upnp-org:metadata-1-0/upnp/" xmlns:dlna="urn:schemas-dlna-org:metadata-1-0/"&gt;&lt;container id="Channels" parentID="0" restricted="1"&gt;&lt;dc:title&gt;Channels&lt;/dc:title&gt;&lt;upnp:class&gt;object.container&lt;/upnp:class&gt;&lt;dlna:containerType&gt;Tuner_1_0&lt;/dlna:containerType&gt;&lt;/container&gt;&lt;item id="v1" parentID="0" restricted="1"&gt;&lt;dc:title&gt;%s&lt;/dc:title&gt;&lt;upnp:class&gt;object.item.videoItem.videoBroadcast&lt;/upnp:class&gt;&lt;upnp:channelNr&gt;%s&lt;/upnp:channelNr&gt;&lt;upnp:channelName&gt;%s&lt;/upnp:channelName&gt;&lt;res protocolInfo="http-get:*:video/mpeg:DLNA.ORG_PN=AVC_TS_HD_60_AC3_ISO;DLNA.ORG_OP=00;DLNA.ORG_FLAGS=01700000000000000000000000000000"&gt;http://%s:%d/auto/v%s&lt;/res&gt;&lt;res protocolInfo="http-get:*:video/mp2t:*"&gt;http://%s:%d/auto/v%s&lt;/res&gt;&lt;res protocolInfo="http-get:*:video/vnd.dlna.mpeg-tts:DLNA.ORG_PN=AVC_TS_NA_ISO;DLNA.ORG_OP=00;DLNA.ORG_FLAGS=01700000000000000000000000000000"&gt;http://%s:%d/auto/v%s&lt;/res&gt;&lt;/item&gt;&lt;/DIDL-Lite&gt;`,
		friendlyName, channelNumber, friendlyName, hostIP, httpPort, channelNumber, hostIP, httpPort, channelNumber, hostIP, httpPort, channelNumber)
}

func handleDiscover(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	jsonResp := fmt.Sprintf(`{"FriendlyName":"%s","ModelNumber":"HDTV-1.0","FirmwareName":"v_atsc_tuner","FirmwareVersion":"20260101","DeviceID":"107BDESK","DeviceAuth":"desktop","BaseURL":"http://%s:%d","LineupURL":"http://%s:%d/lineup.json","TunerCount":2}`,
		friendlyName, hostIP, httpPort, hostIP, httpPort)
	w.Write([]byte(jsonResp))
}

func handleLineup(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	jsonResp := fmt.Sprintf(`[{"GuideNumber":"%s","GuideName":"%s","URL":"http://%s:%d/auto/v%s"}]`,
		channelNumber, friendlyName, hostIP, httpPort, channelNumber)
	w.Write([]byte(jsonResp))
}

func handleDeviceXML(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/xml; charset=utf-8")
	w.Write([]byte(getDeviceXML()))
}

func handleConnectionManager(w http.ResponseWriter, r *http.Request) {
	if r.Method == http.MethodPost {
		body, _ := io.ReadAll(r.Body)
		s := string(body)
		w.Header().Set("Content-Type", "text/xml; charset=utf-8")
		if strings.Contains(s, "GetProtocolInfo") {
			w.Write([]byte(`<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
<s:Body>
<u:GetProtocolInfoResponse xmlns:u="urn:schemas-upnp-org:service:ConnectionManager:1">
<Source>http-get:*:video/mpeg:DLNA.ORG_PN=AVC_TS_HD_60_AC3_ISO;DLNA.ORG_OP=00;DLNA.ORG_FLAGS=01700000000000000000000000000000,http-get:*:video/mpeg:*,http-get:*:video/mp2t:*,http-get:*:video/vnd.dlna.mpeg-tts:*</Source>
<Sink></Sink>
</u:GetProtocolInfoResponse>
</s:Body>
</s:Envelope>`))
		} else {
			w.Write([]byte(`<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
<s:Body>
<u:GetCurrentConnectionIDsResponse xmlns:u="urn:schemas-upnp-org:service:ConnectionManager:1">
<ConnectionIDs>0</ConnectionIDs>
</u:GetCurrentConnectionIDsResponse>
</s:Body>
</s:Envelope>`))
		}
		return
	}
	w.Header().Set("Content-Type", "text/xml; charset=utf-8")
	w.Write([]byte(connectionManagerXML))
}

func handleContentDirectory(w http.ResponseWriter, r *http.Request) {
	if r.Method == http.MethodPost {
		body, _ := io.ReadAll(r.Body)
		s := string(body)
		w.Header().Set("Content-Type", "text/xml; charset=utf-8")
		if strings.Contains(s, "GetSystemUpdateID") {
			w.Write([]byte(`<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
<s:Body><u:GetSystemUpdateIDResponse xmlns:u="urn:schemas-upnp-org:service:ContentDirectory:1"><Id>1</Id></u:GetSystemUpdateIDResponse></s:Body>
</s:Envelope>`))
		} else if strings.Contains(s, "GetSortCapabilities") {
			w.Write([]byte(`<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
<s:Body><u:GetSortCapabilitiesResponse xmlns:u="urn:schemas-upnp-org:service:ContentDirectory:1"><SortCaps></SortCaps></u:GetSortCapabilitiesResponse></s:Body>
</s:Envelope>`))
		} else if strings.Contains(s, "GetSearchCapabilities") {
			w.Write([]byte(`<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
<s:Body><u:GetSearchCapabilitiesResponse xmlns:u="urn:schemas-upnp-org:service:ContentDirectory:1"><SearchCaps></SearchCaps></u:GetSearchCapabilitiesResponse></s:Body>
</s:Envelope>`))
		} else if strings.Contains(s, "Browse") {
			result := getDIDLItem()
			count := 1
			if strings.Contains(s, "ObjectID>0<") || strings.Contains(s, "ObjectID&gt;0&lt;") {
				result = getDIDLContainers()
				count = 2
			}
			resp := fmt.Sprintf(`<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
<s:Body>
<u:BrowseResponse xmlns:u="urn:schemas-upnp-org:service:ContentDirectory:1">
<Result>%s</Result>
<NumberReturned>%d</NumberReturned>
<TotalMatches>%d</TotalMatches>
<UpdateID>1</UpdateID>
</u:BrowseResponse>
</s:Body>
</s:Envelope>`, result, count, count)
			w.Write([]byte(resp))
		} else {
			w.Write([]byte(`<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><s:Fault><faultcode>s:Client</faultcode><faultstring>UPnPError</faultstring></s:Fault></s:Body></s:Envelope>`))
		}
		return
	}
	w.Header().Set("Content-Type", "text/xml; charset=utf-8")
	w.Write([]byte(contentDirectoryXML))
}

func handleStream(w http.ResponseWriter, r *http.Request) {
	// Send DLNA live streaming headers
	w.Header().Set("Server", "Linux/UPnP/1.0 DLNADOC/1.50 VirtualHDTV/1.0")
	w.Header().Set("Connection", "close")
	w.Header().Set("Content-Type", "video/mpeg")
	w.Header().Set("Cache-Control", "no-cache")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	w.Header().Set("transferMode.dlna.org", "Streaming")
	w.Header().Set("contentFeatures.dlna.org", "DLNA.ORG_PN=AVC_TS_HD_60_AC3_ISO;DLNA.ORG_OP=00;DLNA.ORG_FLAGS=01700000000000000000000000000000")

	if r.Method == http.MethodHead {
		w.WriteHeader(http.StatusOK)
		return
	}

	log.Printf("[Stream] CONNECT from %s for %s", r.RemoteAddr, r.URL.Path)
	startTime := time.Now()

	// Audio parameters
	audioArgs := []string{"-c:a", "copy"}
	if audioCodec != "copy" {
		audioArgs = []string{"-c:a", audioCodec, "-b:a", audioBitrate}
	}

	// Remux RTSP to ATSC MPEG-TS with inline SPS/PPS and clock buffer
	ffmpegArgs := []string{
		"-hide_banner",
		"-loglevel", "warning",
		"-rtsp_transport", "tcp",
		"-analyzeduration", "500000",
		"-probesize", "1000000",
		"-i", rtspSource,
		"-c:v", "copy",
		"-bsf:v", "dump_extra",
	}
	ffmpegArgs = append(ffmpegArgs, audioArgs...)
	ffmpegArgs = append(ffmpegArgs,
		"-muxdelay", halfBuffer,
		"-muxpreload", halfBuffer,
		"-pat_period", "0.1",
		"-mpegts_flags", "+initial_discontinuity+resend_headers",
		"-f", "mpegts",
		"pipe:1",
	)

	ctx, cancel := context.WithCancel(r.Context())
	defer cancel()

	cmd := exec.CommandContext(ctx, "ffmpeg", ffmpegArgs...)
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		log.Printf("[Stream] Failed to open FFmpeg stdout pipe: %v", err)
		http.Error(w, "Failed to start stream", http.StatusInternalServerError)
		return
	}

	if err := cmd.Start(); err != nil {
		log.Printf("[Stream] Failed to start FFmpeg: %v", err)
		http.Error(w, "Failed to start stream", http.StatusInternalServerError)
		return
	}

	// Write 200 OK header before streaming body
	w.WriteHeader(http.StatusOK)

	buf := make([]byte, 65536)
	bytesSent, copyErr := io.CopyBuffer(w, stdout, buf)

	// Clean up child process
	cancel()
	_ = cmd.Wait()

	dur := time.Since(startTime).Seconds()
	log.Printf("[Stream] DISCONNECT from %s: sent %d bytes over %.1fs (%v)", r.RemoteAddr, bytesSent, dur, copyErr)
}

func runSSDP() {
	mcastAddr, err := net.ResolveUDPAddr("udp4", "239.255.255.250:1900")
	if err != nil {
		log.Fatalf("[SSDP] Resolve error: %v", err)
	}

	targets := []string{
		"upnp:rootdevice",
		fmt.Sprintf("uuid:%s", deviceUUID),
		"urn:schemas-upnp-org:device:MediaServer:1",
		"urn:schemas-upnp-org:service:ContentDirectory:1",
		"urn:schemas-upnp-org:service:ConnectionManager:1",
	}

	// Send NOTIFY announcement
	sendNotify := func() {
		sendConn, err := net.ListenUDP("udp4", nil)
		if err != nil {
			return
		}
		defer sendConn.Close()

		for _, t := range targets {
			usn := fmt.Sprintf("uuid:%s::%s", deviceUUID, t)
			if t == fmt.Sprintf("uuid:%s", deviceUUID) {
				usn = fmt.Sprintf("uuid:%s", deviceUUID)
			}
			msg := fmt.Sprintf("NOTIFY * HTTP/1.1\r\n"+
				"HOST: 239.255.255.250:1900\r\n"+
				"NT: %s\r\n"+
				"NTS: ssdp:alive\r\n"+
				"LOCATION: http://%s:%d/dms/device.xml\r\n"+
				"USN: %s\r\n"+
				"CACHE-CONTROL: max-age=1800\r\n"+
				"SERVER: Linux/UPnP/1.0 DLNADOC/1.50 VirtualHDTV/1.0\r\n\r\n",
				t, hostIP, httpPort, usn)
			sendConn.WriteTo([]byte(msg), mcastAddr)
		}
	}

	// Initial announcement
	sendNotify()

	// Periodic announcement every 60s
	go func() {
		ticker := time.NewTicker(60 * time.Second)
		defer ticker.Stop()
		for range ticker.C {
			sendNotify()
		}
	}()

	// Listen for M-SEARCH multicast queries
	listener, err := net.ListenMulticastUDP("udp4", nil, mcastAddr)
	if err != nil {
		log.Printf("[SSDP] Multicast listen error: %v (SSDP M-SEARCH answering disabled)", err)
		return
	}
	defer listener.Close()

	log.Printf("[SSDP] Listening on 239.255.255.250:1900...")

	buf := make([]byte, 2048)
	for {
		n, clientAddr, err := listener.ReadFrom(buf)
		if err != nil {
			break
		}
		text := string(buf[:n])
		if !strings.Contains(text, "M-SEARCH") {
			continue
		}

		st := ""
		lines := strings.Split(text, "\r\n")
		for _, line := range lines {
			if strings.HasPrefix(strings.ToUpper(line), "ST:") {
				parts := strings.SplitN(line, ":", 2)
				if len(parts) == 2 {
					st = strings.TrimSpace(parts[1])
				}
				break
			}
		}

		if st == "" {
			continue
		}

		var matched []string
		if st == "ssdp:all" {
			matched = targets
		} else {
			for _, t := range targets {
				if st == t {
					matched = append(matched, t)
				}
			}
		}

		for _, m := range matched {
			usn := fmt.Sprintf("uuid:%s::%s", deviceUUID, m)
			if m == fmt.Sprintf("uuid:%s", deviceUUID) {
				usn = fmt.Sprintf("uuid:%s", deviceUUID)
			}
			resp := fmt.Sprintf("HTTP/1.1 200 OK\r\n"+
				"CACHE-CONTROL: max-age=1800\r\n"+
				"DATE: %s\r\n"+
				"EXT:\r\n"+
				"LOCATION: http://%s:%d/dms/device.xml\r\n"+
				"SERVER: Linux/UPnP/1.0 DLNADOC/1.50 VirtualHDTV/1.0\r\n"+
				"ST: %s\r\n"+
				"USN: %s\r\n\r\n",
				time.Now().UTC().Format(time.RFC1123), hostIP, httpPort, m, usn)
			listener.WriteTo([]byte(resp), clientAddr)
		}
	}
}

func startMediaMTX() *exec.Cmd {
	binPath := "/app/mediamtx"
	if env := os.Getenv("MEDIAMTX_BIN"); env != "" {
		binPath = env
	}
	configPath := "/app/mediamtx.yml"
	if env := os.Getenv("MEDIAMTX_CONFIG"); env != "" {
		configPath = env
	}

	if _, err := os.Stat(binPath); err == nil {
		log.Printf("[Init] Starting embedded MediaMTX engine (%s)...", binPath)
		cmd := exec.Command(binPath, configPath)
		cmd.Stdout = os.Stdout
		cmd.Stderr = os.Stderr
		if err := cmd.Start(); err != nil {
			log.Printf("[Init] Warning: Failed to start MediaMTX: %v", err)
			return nil
		}
		return cmd
	}
	return nil
}

func main() {
	initConfig()

	log.Printf("==================================================")
	log.Printf("       Starting OBS HomeRun (Go Engine)           ")
	log.Printf("==================================================")
	log.Printf("Friendly Name:   %s", friendlyName)
	log.Printf("Channel Number:  %s", channelNumber)
	log.Printf("Host Address:    %s:%d", hostIP, httpPort)
	log.Printf("RTSP Source:     %s", rtspSource)
	log.Printf("Buffer Safety:   %.1fs (muxdelay: %ss)", bufferSec, halfBuffer)
	log.Printf("Audio Codec:     %s (%s)", audioCodec, audioBitrate)
	log.Printf("==================================================")

	mtxCmd := startMediaMTX()
	if mtxCmd != nil {
		defer func() {
			if mtxCmd.Process != nil {
				log.Println("[Init] Terminating MediaMTX process...")
				_ = mtxCmd.Process.Signal(syscall.SIGTERM)
				_ = mtxCmd.Wait()
			}
		}()
	}

	mux := http.NewServeMux()
	mux.HandleFunc("/discover.json", handleDiscover)
	mux.HandleFunc("/lineup.json", handleLineup)
	mux.HandleFunc("/dms/device.xml", handleDeviceXML)
	mux.HandleFunc("/dms/ConnectionManager.xml", handleConnectionManager)
	mux.HandleFunc("/dms/ConnectionManager", handleConnectionManager)
	mux.HandleFunc("/dms/ContentDirectory.xml", handleContentDirectory)
	mux.HandleFunc("/dms/ContentDirectory", handleContentDirectory)
	mux.HandleFunc("/auto/", handleStream)
	mux.HandleFunc("/live", handleStream)
	mux.HandleFunc("/stream.ts", handleStream)

	server := &http.Server{
		Addr:    fmt.Sprintf(":%d", httpPort),
		Handler: mux,
	}

	go runSSDP()

	go func() {
		if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			log.Fatalf("HTTP server failed: %v", err)
		}
	}()

	// Graceful shutdown on SIGINT / SIGTERM
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, os.Interrupt, syscall.SIGTERM)
	<-sigChan

	log.Println("Shutting down OBS HomeRun...")
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()
	server.Shutdown(ctx)
	log.Println("OBS HomeRun exited.")
}
