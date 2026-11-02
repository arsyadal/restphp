<?php

namespace RestPhp\Octane;

use Illuminate\Support\ServiceProvider;
use RestPhp\Octane\Commands\StartRestPhpCommand;

class RestPhpServiceProvider extends ServiceProvider
{
    public function register(): void
    {
        $this->app->singleton(RestPhpServerProcessInspector::class, fn ($app) =>
            new RestPhpServerProcessInspector($app->storagePath('logs/restphp-server-state.json'))
        );
    }

    public function boot(): void
    {
        if ($this->app->runningInConsole()) {
            $this->commands([StartRestPhpCommand::class]);
        }
    }
}
