# Use the official Node.js image as a base
FROM node:20-alpine AS deps

# Set the working directory
WORKDIR /app

# Copy package.json and pnpm-lock.yaml
COPY quark-webhook/package.json quark-webhook/pnpm-lock.yaml ./

# Install pnpm
RUN npm install -g pnpm

# Install dependencies
RUN pnpm install --frozen-lockfile

# Copy the rest of the application code
COPY quark-webhook/ .

# Build the application
RUN pnpm build

# Create a new, smaller image for the final application
FROM node:20-alpine AS runner

WORKDIR /app

# Install pnpm in the runtime image
RUN npm install -g pnpm

# Copy the build output from the builder stage
COPY --from=deps /app/.next ./.next
COPY --from=deps /app/node_modules ./node_modules
COPY --from=deps /app/package.json ./package.json
COPY --from=deps /app/public ./public

# Expose the port the app runs on
EXPOSE 3000

# Run the application
CMD ["pnpm", "start"] 