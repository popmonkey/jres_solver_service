const path = require('path');

module.exports = {
  apps: [
    {
      name: 'jres_solver',
      script: './target/release/jres_solver_service',
      cwd: __dirname,
      instances: 1,
      autorestart: true,
      watch: false,
      max_memory_restart: '1G',
      env: {
        RUST_LOG: 'info',
        LOG_DIR: path.join(__dirname, 'service', 'logs')
      },
      error_file: path.join(__dirname, 'service', 'logs', 'pm2.err.log'),
      out_file: path.join(__dirname, 'service', 'logs', 'pm2.out.log'),
      log_date_format: 'YYYY-MM-DD HH:mm:ss'
    }
  ]
};