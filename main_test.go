package main

import (
	"encoding/json"
	"encoding/xml"
	"io"
	"net"
	"net/http"
	"net/http/httptest"
	"os"
	"os/exec"
	"regexp"
	"strings"
	"testing"
)

func TestUUIDGeneration(t *testing.T) {
	uuid1 := generateUUID("OBS HomeRun")
	uuid2 := generateUUID("OBS HomeRun")
	uuid3 := generateUUID("Other Name")

	if uuid1 != uuid2 {
		t.Errorf("Expected deterministic UUIDs for same name, got %s and %s", uuid1, uuid2)
	}
	if uuid1 == uuid3 {
		t.Errorf("Expected different UUIDs for different names, got identical %s", uuid1)
	}

	// Verify RFC 4122 format: 8-4-4-4-12 hex characters
	uuidRegex := regexp.MustCompile(`^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$`)
	if !uuidRegex.MatchString(uuid1) {
		t.Errorf("UUID %s does not match RFC 4122 pattern", uuid1)
	}
}

func TestInitConfigDefaults(t *testing.T) {
	// Clear any overrides for test
	os.Unsetenv("FRIENDLY_NAME")
	os.Unsetenv("CHANNEL_NUMBER")
	os.Unsetenv("BUFFER_SECONDS")
	os.Unsetenv("AUDIO_CODEC")
	os.Unsetenv("HTTP_PORT")

	initConfig()

	if friendlyName != "OBS HomeRun" {
		t.Errorf("Expected friendlyName 'OBS HomeRun', got '%s'", friendlyName)
	}
	if channelNumber != "1.1" {
		t.Errorf("Expected channelNumber '1.1', got '%s'", channelNumber)
	}
	if bufferSec != 3.0 {
		t.Errorf("Expected bufferSec 3.0, got %f", bufferSec)
	}
	if halfBuffer != "1.5" {
		t.Errorf("Expected halfBuffer '1.5', got '%s'", halfBuffer)
	}
	if audioCodec != "ac3" {
		t.Errorf("Expected audioCodec 'ac3', got '%s'", audioCodec)
	}
	if httpPort != 5004 {
		t.Errorf("Expected httpPort 5004, got %d", httpPort)
	}
}

func TestHandleDiscoverJSON(t *testing.T) {
	initConfig()

	req := httptest.NewRequest(http.MethodGet, "/discover.json", nil)
	w := httptest.NewRecorder()

	handleDiscover(w, req)

	resp := w.Result()
	if resp.StatusCode != http.StatusOK {
		t.Fatalf("Expected status 200, got %d", resp.StatusCode)
	}
	if ct := resp.Header.Get("Content-Type"); !strings.Contains(ct, "application/json") {
		t.Errorf("Expected application/json Content-Type, got '%s'", ct)
	}

	var data map[string]interface{}
	body, _ := io.ReadAll(resp.Body)
	if err := json.Unmarshal(body, &data); err != nil {
		t.Fatalf("Failed to parse discover.json response: %v", err)
	}

	if data["FriendlyName"] != friendlyName {
		t.Errorf("Expected FriendlyName '%s', got '%v'", friendlyName, data["FriendlyName"])
	}
	if data["ModelNumber"] != "HDTV-1.0" {
		t.Errorf("Expected ModelNumber 'HDTV-1.0', got '%v'", data["ModelNumber"])
	}
	if data["TunerCount"] != float64(2) {
		t.Errorf("Expected TunerCount 2, got '%v'", data["TunerCount"])
	}
}

func TestHandleLineupJSON(t *testing.T) {
	initConfig()

	req := httptest.NewRequest(http.MethodGet, "/lineup.json", nil)
	w := httptest.NewRecorder()

	handleLineup(w, req)

	resp := w.Result()
	if resp.StatusCode != http.StatusOK {
		t.Fatalf("Expected status 200, got %d", resp.StatusCode)
	}

	var lineup []map[string]string
	body, _ := io.ReadAll(resp.Body)
	if err := json.Unmarshal(body, &lineup); err != nil {
		t.Fatalf("Failed to parse lineup.json response: %v", err)
	}

	if len(lineup) != 1 {
		t.Fatalf("Expected 1 lineup channel, got %d", len(lineup))
	}
	if lineup[0]["GuideNumber"] != channelNumber {
		t.Errorf("Expected GuideNumber '%s', got '%s'", channelNumber, lineup[0]["GuideNumber"])
	}
	if lineup[0]["GuideName"] != friendlyName {
		t.Errorf("Expected GuideName '%s', got '%s'", friendlyName, lineup[0]["GuideName"])
	}
}

func TestHandleDeviceXML(t *testing.T) {
	initConfig()

	req := httptest.NewRequest(http.MethodGet, "/dms/device.xml", nil)
	w := httptest.NewRecorder()

	handleDeviceXML(w, req)

	resp := w.Result()
	if resp.StatusCode != http.StatusOK {
		t.Fatalf("Expected status 200, got %d", resp.StatusCode)
	}
	if ct := resp.Header.Get("Content-Type"); !strings.Contains(ct, "text/xml") {
		t.Errorf("Expected text/xml Content-Type, got '%s'", ct)
	}

	body, _ := io.ReadAll(resp.Body)
	var parsed struct {
		XMLName xml.Name `xml:"root"`
		Device  struct {
			FriendlyName string `xml:"friendlyName"`
			UDN          string `xml:"UDN"`
		} `xml:"device"`
	}
	if err := xml.Unmarshal(body, &parsed); err != nil {
		t.Fatalf("Failed to parse device.xml XML: %v\nBody was:\n%s", err, string(body))
	}

	if parsed.Device.FriendlyName != friendlyName {
		t.Errorf("Expected FriendlyName '%s', got '%s'", friendlyName, parsed.Device.FriendlyName)
	}
	if !strings.HasPrefix(parsed.Device.UDN, "uuid:") {
		t.Errorf("Expected UDN to start with uuid:, got '%s'", parsed.Device.UDN)
	}
}

func TestHandleConnectionManagerSOAP(t *testing.T) {
	initConfig()

	// 1. GET returns SCPD XML
	reqGet := httptest.NewRequest(http.MethodGet, "/dms/ConnectionManager.xml", nil)
	wGet := httptest.NewRecorder()
	handleConnectionManager(wGet, reqGet)
	if wGet.Result().StatusCode != http.StatusOK {
		t.Errorf("Expected GET ConnectionManager.xml 200, got %d", wGet.Result().StatusCode)
	}

	// 2. POST GetProtocolInfo
	soapBody := `<?xml version="1.0"?><s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><u:GetProtocolInfo xmlns:u="urn:schemas-upnp-org:service:ConnectionManager:1"/></s:Body></s:Envelope>`
	reqPost := httptest.NewRequest(http.MethodPost, "/dms/ConnectionManager", strings.NewReader(soapBody))
	wPost := httptest.NewRecorder()
	handleConnectionManager(wPost, reqPost)

	respPost := wPost.Result()
	body, _ := io.ReadAll(respPost.Body)
	if !strings.Contains(string(body), "GetProtocolInfoResponse") {
		t.Errorf("Expected GetProtocolInfoResponse in body, got:\n%s", string(body))
	}
	if !strings.Contains(string(body), "video/mpeg") {
		t.Errorf("Expected video/mpeg in protocolInfo, got:\n%s", string(body))
	}
}

func TestHandleContentDirectorySOAP(t *testing.T) {
	initConfig()

	// 1. POST Browse Root Container (ObjectID = 0)
	soapBrowseRoot := `<?xml version="1.0"?><s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><u:Browse xmlns:u="urn:schemas-upnp-org:service:ContentDirectory:1"><ObjectID>0</ObjectID><BrowseFlag>BrowseDirectChildren</BrowseFlag></u:Browse></s:Body></s:Envelope>`
	reqRoot := httptest.NewRequest(http.MethodPost, "/dms/ContentDirectory", strings.NewReader(soapBrowseRoot))
	wRoot := httptest.NewRecorder()
	handleContentDirectory(wRoot, reqRoot)

	bodyRoot, _ := io.ReadAll(wRoot.Result().Body)
	if !strings.Contains(string(bodyRoot), "NumberReturned&gt;2&lt;/NumberReturned") && !strings.Contains(string(bodyRoot), "<NumberReturned>2</NumberReturned>") {
		t.Errorf("Expected NumberReturned 2 for root browse, got:\n%s", string(bodyRoot))
	}
	if !strings.Contains(string(bodyRoot), "Tuner_1_0") {
		t.Errorf("Expected Tuner_1_0 container in root browse, got:\n%s", string(bodyRoot))
	}

	// 2. POST Browse Item
	soapBrowseItem := `<?xml version="1.0"?><s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><u:Browse xmlns:u="urn:schemas-upnp-org:service:ContentDirectory:1"><ObjectID>v1</ObjectID><BrowseFlag>BrowseMetadata</BrowseFlag></u:Browse></s:Body></s:Envelope>`
	reqItem := httptest.NewRequest(http.MethodPost, "/dms/ContentDirectory", strings.NewReader(soapBrowseItem))
	wItem := httptest.NewRecorder()
	handleContentDirectory(wItem, reqItem)

	bodyItem, _ := io.ReadAll(wItem.Result().Body)
	if !strings.Contains(string(bodyItem), "object.item.videoItem.videoBroadcast") {
		t.Errorf("Expected object.item.videoItem.videoBroadcast in item browse, got:\n%s", string(bodyItem))
	}
	if !strings.Contains(string(bodyItem), "DLNA.ORG_OP=00") {
		t.Errorf("Expected DLNA.ORG_OP=00 (live stream without seeking) in item browse, got:\n%s", string(bodyItem))
	}
}

func TestHandleStreamHeaders(t *testing.T) {
	initConfig()

	// Test HEAD request for DLNA stream probe
	req := httptest.NewRequest(http.MethodHead, "/auto/v1.1", nil)
	w := httptest.NewRecorder()

	handleStream(w, req)

	resp := w.Result()
	if resp.StatusCode != http.StatusOK {
		t.Fatalf("Expected status 200, got %d", resp.StatusCode)
	}

	expectedHeaders := map[string]string{
		"Content-Type":          "video/mpeg",
		"transferMode.dlna.org": "Streaming",
		"Connection":            "close",
	}

	for header, expected := range expectedHeaders {
		if val := resp.Header.Get(header); val != expected {
			t.Errorf("Header '%s': expected '%s', got '%s'", header, expected, val)
		}
	}

	cf := resp.Header.Get("contentFeatures.dlna.org")
	if !strings.Contains(cf, "DLNA.ORG_OP=00") {
		t.Errorf("Expected DLNA.ORG_OP=00 in contentFeatures, got '%s'", cf)
	}
	if !strings.Contains(cf, "AVC_TS_HD_60_AC3_ISO") {
		t.Errorf("Expected AVC_TS_HD_60_AC3_ISO in contentFeatures, got '%s'", cf)
	}
}

func TestE2EStreamPipeline(t *testing.T) {
	// Skip if ffmpeg is not available on host
	if _, err := exec.LookPath("ffmpeg"); err != nil {
		t.Skip("ffmpeg not installed on system; skipping E2E pipeline test")
	}

	initConfig()

	// Create test HTTP server
	server := httptest.NewServer(http.HandlerFunc(handleStream))
	defer server.Close()

	// Connect to /auto/v1.1 with HEAD request
	resp, err := http.Head(server.URL + "/auto/v1.1")
	if err != nil {
		t.Fatalf("Failed to make HEAD request to stream endpoint: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Fatalf("Expected HTTP 200 OK, got %d", resp.StatusCode)
	}
	if resp.Header.Get("Content-Type") != "video/mpeg" {
		t.Errorf("Expected Content-Type video/mpeg, got %s", resp.Header.Get("Content-Type"))
	}
}

func TestDefaultHostIP(t *testing.T) {
	ip := getDefaultHostIP()
	if ip == "" {
		t.Fatal("Expected non-empty host IP")
	}
	// Verify it's a valid IPv4 address
	parsed := net.ParseIP(ip)
	if parsed == nil {
		t.Fatalf("Failed to parse default host IP '%s'", ip)
	}
}
