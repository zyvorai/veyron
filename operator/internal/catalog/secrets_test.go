// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

package catalog

import (
	"encoding/xml"
	"strings"
	"testing"
)

const joinPassword = "Sup3r$ecret&<pw>"

func testCreds() *DomainJoinCredentials {
	return &DomainJoinCredentials{
		Domain:   "corp.example.com",
		OU:       "OU=Servers,DC=corp,DC=example,DC=com",
		Username: `CORP\joinsvc`,
		Password: joinPassword,
	}
}

// The answer file must be well-formed XML even when the password contains the XML
// metacharacters that show up in real generated passwords. An unescaped `&` produces
// a file Windows silently refuses, stranding Setup at an interactive prompt.
func TestBuildDomainJoinUnattend_IsWellFormedAndEscapesPassword(t *testing.T) {
	out := BuildDomainJoinUnattend(testCreds())

	if err := xml.Unmarshal([]byte(out), new(interface{})); err != nil {
		t.Fatalf("answer file is not well-formed XML: %v\n%s", err, out)
	}
	if strings.Contains(out, joinPassword) {
		t.Fatal("password was written raw; XML metacharacters must be escaped")
	}
	if !strings.Contains(out, "Sup3r$ecret&amp;&lt;pw&gt;") {
		t.Fatalf("password not escaped as expected:\n%s", out)
	}
	if !strings.Contains(out, "Microsoft-Windows-UnattendedJoin") {
		t.Fatal("missing UnattendedJoin component")
	}
	if !strings.Contains(out, "<JoinDomain>corp.example.com</JoinDomain>") {
		t.Fatal("missing JoinDomain")
	}
	if !strings.Contains(out, "<MachineObjectOU>OU=Servers,DC=corp,DC=example,DC=com</MachineObjectOU>") {
		t.Fatal("missing MachineObjectOU")
	}
}

// `CORP\joinsvc` must be split: unattend wants the account and its domain in
// separate elements. Passing the qualified form through fails with a bare
// "access denied" that is near-impossible to diagnose from inside the guest.
func TestSplitDomainUser(t *testing.T) {
	cases := []struct{ in, wantUser, wantDomain string }{
		{`CORP\joinsvc`, "joinsvc", "CORP"},
		{"joinsvc@corp.example.com", "joinsvc", "corp.example.com"},
		{"joinsvc", "joinsvc", "corp.example.com"}, // falls back to the join domain
	}
	for _, c := range cases {
		u, d := splitDomainUser(c.in, "corp.example.com")
		if u != c.wantUser || d != c.wantDomain {
			t.Errorf("splitDomainUser(%q) = (%q,%q), want (%q,%q)", c.in, u, d, c.wantUser, c.wantDomain)
		}
	}
}

// A caller's own answer file must survive: the join is injected into their existing
// specialize pass, not substituted for their file.
func TestMergeDomainJoinIntoUnattend_ReusesExistingSpecializePass(t *testing.T) {
	userXML := `<?xml version="1.0" encoding="utf-8"?>
<unattend xmlns="urn:schemas-microsoft-com:unattend">
  <settings pass="specialize">
    <component name="Microsoft-Windows-Shell-Setup">
      <ComputerName>WIN01</ComputerName>
    </component>
  </settings>
</unattend>
`
	out, err := MergeDomainJoinIntoUnattend(userXML, testCreds())
	if err != nil {
		t.Fatalf("merge failed: %v", err)
	}
	if err := xml.Unmarshal([]byte(out), new(interface{})); err != nil {
		t.Fatalf("merged answer file is not well-formed XML: %v\n%s", err, out)
	}
	if !strings.Contains(out, "<ComputerName>WIN01</ComputerName>") {
		t.Fatal("caller's own config was discarded")
	}
	if !strings.Contains(out, "Microsoft-Windows-UnattendedJoin") {
		t.Fatal("domain join was not injected")
	}
	if strings.Count(out, `pass="specialize"`) != 1 {
		t.Fatalf("should reuse the existing specialize pass, not add a second:\n%s", out)
	}
}

// An answer file with no specialize pass still needs one for the join.
func TestMergeDomainJoinIntoUnattend_AddsSpecializePassWhenAbsent(t *testing.T) {
	userXML := `<?xml version="1.0" encoding="utf-8"?>
<unattend xmlns="urn:schemas-microsoft-com:unattend">
  <settings pass="oobeSystem">
    <component name="Microsoft-Windows-Shell-Setup">
      <TimeZone>UTC</TimeZone>
    </component>
  </settings>
</unattend>
`
	out, err := MergeDomainJoinIntoUnattend(userXML, testCreds())
	if err != nil {
		t.Fatalf("merge failed: %v", err)
	}
	if err := xml.Unmarshal([]byte(out), new(interface{})); err != nil {
		t.Fatalf("merged answer file is not well-formed XML: %v\n%s", err, out)
	}
	if !strings.Contains(out, "<TimeZone>UTC</TimeZone>") {
		t.Fatal("caller's oobeSystem pass was discarded")
	}
	if !strings.Contains(out, `pass="specialize"`) {
		t.Fatal("specialize pass was not added")
	}
}

// Two competing domain joins is a config error, not something to silently pick from.
func TestMergeDomainJoinIntoUnattend_RejectsExistingJoin(t *testing.T) {
	userXML := `<unattend xmlns="urn:schemas-microsoft-com:unattend">
  <settings pass="specialize">
    <component name="Microsoft-Windows-UnattendedJoin"></component>
  </settings>
</unattend>`
	if _, err := MergeDomainJoinIntoUnattend(userXML, testCreds()); err == nil {
		t.Fatal("expected an error when the answer file already joins a domain")
	}
}

// Garbage in must not produce a broken answer file that strands Setup at a prompt.
func TestMergeDomainJoinIntoUnattend_RejectsNonUnattendDocument(t *testing.T) {
	if _, err := MergeDomainJoinIntoUnattend("this is not xml", testCreds()); err == nil {
		t.Fatal("expected an error for a document with no </unattend>")
	}
}
