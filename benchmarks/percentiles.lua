local threads = {}

function setup(thread)
    table.insert(threads, thread)
end

function init(args)
    non_2xx = 0
end

-- The response callback cost is identical for both runtimes. Unlike wrk's
-- built-in status counter, this also counts redirects as non-2xx responses.
function response(status, headers, body)
    if status < 200 or status >= 300 then
        non_2xx = non_2xx + 1
    end
end

function done(summary, latency, requests)
    for _, percentile in ipairs({50, 95, 99}) do
        io.write(string.format("  %d%% %.6fus\n", percentile, latency:percentile(percentile)))
    end
    local errors = 0
    for _, thread in ipairs(threads) do
        errors = errors + thread:get("non_2xx")
    end
    io.write(string.format("Benchmark non-2xx responses: %d\n", errors))
end
