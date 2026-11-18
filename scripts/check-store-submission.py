#!/usr/bin/env python3
"""Reject Store publication if another submission is pending."""

import json
import os
import sys
import urllib.error
import urllib.parse
import urllib.request

PRODUCT_ID = "9NBG48TV6NP3"


def check_application(application):
    if application.get("id") != PRODUCT_ID:
        raise ValueError("Partner Center returned an unexpected product.")
    if not application.get("lastPublishedApplicationSubmission"):
        raise ValueError("No previously published submission was found.")
    if application.get("pendingApplicationSubmission") is not None:
        raise ValueError("A Store submission is already pending; resolve it before retrying.")


def main():
    names = ("PARTNER_CENTER_TENANT_ID", "PARTNER_CENTER_CLIENT_ID", "PARTNER_CENTER_CLIENT_SECRET")
    if any(not os.environ.get(name) for name in names):
        raise ValueError("Missing Partner Center credentials in GitHub Actions secrets.")
    tenant = urllib.parse.quote(os.environ[names[0]], safe="")
    payload = urllib.parse.urlencode({
        "grant_type": "client_credentials",
        "client_id": os.environ[names[1]],
        "client_secret": os.environ[names[2]],
        "resource": "https://manage.devcenter.microsoft.com",
    }).encode()
    request = urllib.request.Request(
        f"https://login.microsoftonline.com/{tenant}/oauth2/token", data=payload, method="POST"
    )
    with urllib.request.urlopen(request, timeout=30) as response:
        token = json.load(response)["access_token"]
    request = urllib.request.Request(
        f"https://manage.devcenter.microsoft.com/v1.0/my/applications/{PRODUCT_ID}",
        headers={"Authorization": f"Bearer {token}"},
    )
    with urllib.request.urlopen(request, timeout=30) as response:
        application = json.load(response)
    check_application(application)
    print("No pending Store submission found.")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError, urllib.error.URLError):
        print("Store preflight failed: check credentials, API access, and pending submissions in Partner Center.", file=sys.stderr)
        sys.exit(1)
