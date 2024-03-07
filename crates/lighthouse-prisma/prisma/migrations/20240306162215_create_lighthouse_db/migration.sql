-- CreateTable
CREATE TABLE "Pubkey" (
    "id" SERIAL NOT NULL,
    "endpointID" UUID NOT NULL,
    "pubkey" TEXT NOT NULL,

    CONSTRAINT "Pubkey_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "Endpoint" (
    "id" UUID NOT NULL,
    "ip" INET NOT NULL,
    "port" INTEGER NOT NULL,
    "lastUpdate" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "Endpoint_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "PendingConnection" (
    "id" SERIAL NOT NULL,
    "endpointID" UUID NOT NULL,
    "ip" INET NOT NULL,
    "port" INTEGER NOT NULL,
    "fingerprint" BYTEA NOT NULL,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "PendingConnection_pkey" PRIMARY KEY ("id")
);

-- AddForeignKey
ALTER TABLE "Pubkey" ADD CONSTRAINT "Pubkey_endpointID_fkey" FOREIGN KEY ("endpointID") REFERENCES "Endpoint"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "PendingConnection" ADD CONSTRAINT "PendingConnection_endpointID_fkey" FOREIGN KEY ("endpointID") REFERENCES "Endpoint"("id") ON DELETE RESTRICT ON UPDATE CASCADE;
