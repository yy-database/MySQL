// Nifty project configuration for MySQL (hybrid cargo + pnpm).
import { defineConfig } from '@doki-land/nifty';

export default defineConfig({
    trust: {
        npm: {
            repo: 'yy-database/MySQL',
            file: 'publish-npm.yml',
            environment: 'NPM_PUBLISH',
        },
    },
});
