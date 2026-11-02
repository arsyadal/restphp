<?php

namespace RestPhp\Octane;

use Illuminate\Foundation\Application;
use Illuminate\Http\Request;
use Laravel\Octane\Contracts\StoppableClient;
use Laravel\Octane\Octane;
use Laravel\Octane\OctaneResponse;
use Laravel\Octane\RequestContext;
use Symfony\Component\HttpFoundation\Response;
use Throwable;

class RestPhpClient implements StoppableClient
{
    public function marshalRequest(RequestContext $context): array
    {
        return [Request::capture(), $context];
    }

    public function respond(RequestContext $context, OctaneResponse $response): void
    {
        $response->response->sendHeaders();
        echo $response->outputBuffer ?? '';
        $response->response->sendContent();
    }

    public function error(Throwable $e, Application $app, Request $request, RequestContext $context): void
    {
        (new Response(
            Octane::formatExceptionForClient($e, (bool) $app->make('config')->get('app.debug')),
            500,
            ['Content-Type' => 'text/plain'],
        ))->send();

        // Worker-level failures may leave listeners or singleton services dirty.
        $this->stop();
    }

    public function stop(): void
    {
        restphp_stop_worker();
    }
}
