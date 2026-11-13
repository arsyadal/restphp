import unittest

from run_soak import check_budgets


class ResourceBudgetTests(unittest.TestCase):
    def test_transient_rss_peak_is_not_hidden_by_final_recovery(self):
        samples = [{"rss_kib": 20000, "fd_count": 8},
                   {"rss_kib": 55000, "fd_count": 8},
                   {"rss_kib": 20000, "fd_count": 8}]
        with self.assertRaises(RuntimeError):
            check_budgets(samples, 32768, 16, 262144)

    def test_descriptor_peak_is_not_hidden_by_later_cleanup(self):
        samples = [{"rss_kib": 20000, "fd_count": 8},
                   {"rss_kib": 22000, "fd_count": 25},
                   {"rss_kib": 22000, "fd_count": 8}]
        with self.assertRaises(RuntimeError):
            check_budgets(samples, 32768, 16, 262144)

    def test_absolute_rss_limit_also_applies_to_warm_worker(self):
        with self.assertRaises(RuntimeError):
            check_budgets([{"rss_kib": 300000, "fd_count": 8}], 32768, 16, 262144)

    def test_reports_measured_peak_and_last_values_separately(self):
        result = check_budgets([{"rss_kib": 20000, "fd_count": 8},
                               {"rss_kib": 25000, "fd_count": 10},
                               {"rss_kib": 23000, "fd_count": 9}], 32768, 16, 262144)
        self.assertEqual(result["peak_rss_kib"], 25000)
        self.assertEqual(result["last_rss_kib"], 23000)
        self.assertEqual(result["rss_growth_kib"], 5000)
        self.assertEqual(result["peak_fd_count"], 10)
        self.assertEqual(result["last_fd_count"], 9)

    def test_unmeasured_worker_cannot_pass_resource_validation(self):
        with self.assertRaises(RuntimeError):
            check_budgets([], 32768, 16, 262144)


if __name__ == "__main__":
    unittest.main()
