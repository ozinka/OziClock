#!/usr/bin/env python3
"""Tests for the non-destructive Store submission preflight."""

import importlib.util
from pathlib import Path
import sys
import unittest

sys.dont_write_bytecode = True

spec = importlib.util.spec_from_file_location("store_preflight", Path(__file__).with_name("check-store-submission.py"))
preflight = importlib.util.module_from_spec(spec)
spec.loader.exec_module(preflight)


class StorePreflightTests(unittest.TestCase):
    def test_published_app_without_pending_submission_is_allowed(self):
        preflight.check_application({"id": preflight.PRODUCT_ID, "lastPublishedApplicationSubmission": {"id": "1"}, "pendingApplicationSubmission": None})

    def test_pending_submission_is_rejected(self):
        with self.assertRaises(ValueError):
            preflight.check_application({"id": preflight.PRODUCT_ID, "lastPublishedApplicationSubmission": {"id": "1"}, "pendingApplicationSubmission": {"id": "2"}})

    def test_unexpected_product_or_missing_published_submission_is_rejected(self):
        for app in ({"id": "wrong", "lastPublishedApplicationSubmission": {"id": "1"}}, {"id": preflight.PRODUCT_ID}):
            with self.subTest(app=app), self.assertRaises(ValueError):
                preflight.check_application(app)


if __name__ == "__main__":
    unittest.main()
