<?php

namespace RestPhp\Octane;

use Illuminate\Support\ServiceProvider;
use RestPhp\Octane\Commands\StartRestPhpCommand;

class RestPhpServiceProvider extends ServiceProvider
{
    /**
     * Register any application services.
     */
    public function register(): void
    {
        $this->app->singleton(RestPhpServerProcessInspector::class);
    }

    /**
     * Bootstrap any application services.
     */
    public function boot(): void
    {
        if ($this->app->runningInConsole()) {
            $this->commands([
                StartRestPhpCommand::class,
            ]);
        }
    }
}
