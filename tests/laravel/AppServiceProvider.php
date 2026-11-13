<?php

namespace App\Providers;

use Illuminate\Http\Request;
use Illuminate\Support\Facades\Route;
use Illuminate\Support\ServiceProvider;

class AppServiceProvider extends ServiceProvider
{
    public function register(): void
    {
        $boot = bin2hex(random_bytes(16));
        $this->app->singleton('regression.boot', fn () => $boot);
        $this->app->scoped('regression.scope', fn () => (object) [
            'token' => bin2hex(random_bytes(16)),
            'uses' => 0,
        ]);
    }

    public function boot(): void
    {
        if (PHP_SAPI === 'restphp') {
            file_put_contents(storage_path('logs/regression-boots.jsonl'), json_encode([
                'pid' => getmypid(),
                'boot' => $this->app->make('regression.boot'),
            ], JSON_THROW_ON_ERROR)."\n", FILE_APPEND | LOCK_EX);
        }

        Route::match(['GET', 'POST'], '/regression/probe', function (Request $request) {
            $scope = app('regression.scope');
            $scope->uses++;
            $again = app('regression.scope');
            $again->uses++;
            $response = response()->json([
                'pid' => getmypid(),
                'boot' => app('regression.boot'),
                'scope' => $scope->token,
                'scope_uses' => $scope->uses,
                'scope_same' => $scope === $again,
                'method' => $request->method(),
                'body' => $request->getContent(),
                'input' => $request->input('value'),
                'cookie' => $request->cookie('seed'),
                'header' => $request->header('X-Regression'),
                'php' => PHP_VERSION,
                'laravel' => app()->version(),
                'octane' => \Composer\InstalledVersions::getPrettyVersion('laravel/octane'),
            ], $request->boolean('mutate') ? 201 : 200);
            if ($request->boolean('mutate')) {
                $response->headers->set('X-Regression-Only', 'first');
                $response->cookie('regression', 'first', 0);
            }
            return $response;
        });

        Route::get('/regression/exception', function () {
            throw new \RuntimeException('ordinary Laravel kernel exception');
        });
    }
}
