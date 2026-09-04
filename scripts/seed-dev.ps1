param(
    [string]$BaseUrl = "http://127.0.0.1:8080/api/v1"
)

$ErrorActionPreference = "Stop"

$OwnerEmail = "owner@hamsavilas.test"
$OwnerPassword = "Owner@123"
$ManagerEmail = "manager.koramangala@hamsavilas.test"
$ManagerPassword = "Manager@123"
$TenantEmail = "tenant.arjun@hamsavilas.test"
$TenantPassword = "Tenant@123"
$ProvisioningKey = "hamsa-local-provisioning-key"

function Invoke-HamsaApi {
    param(
        [Parameter(Mandatory = $true)][string]$Method,
        [Parameter(Mandatory = $true)][string]$Path,
        [object]$Body,
        [string]$Token,
        [hashtable]$Headers = @{}
    )

    $requestHeaders = @{}
    foreach ($key in $Headers.Keys) {
        $requestHeaders[$key] = $Headers[$key]
    }
    if ($Token) {
        $requestHeaders["Authorization"] = "Bearer $Token"
    }

    $request = @{
        Uri         = "$BaseUrl$Path"
        Method      = $Method
        Headers     = $requestHeaders
        ContentType = "application/json"
    }
    if ($null -ne $Body) {
        $request["Body"] = $Body | ConvertTo-Json -Depth 12 -Compress
    }

    try {
        Invoke-RestMethod @request
    } catch {
        throw "$Method $Path failed: $($_.Exception.Message)"
    }
}

try {
    $existing = Invoke-HamsaApi -Method Post -Path "/auth/login" -Body @{
        email = $OwnerEmail
        password = $OwnerPassword
    }
    Write-Host "Development data already exists; leaving it unchanged."
    Write-Host "Owner:   $OwnerEmail / $OwnerPassword"
    Write-Host "Manager: $ManagerEmail / $ManagerPassword"
    Write-Host "Tenant:  $TenantEmail / $TenantPassword"
    exit 0
} catch {
    # A fresh database should reject the development owner login.
}

$provisioned = Invoke-HamsaApi -Method Post -Path "/orgs" -Headers @{
    "x-provisioning-key" = $ProvisioningKey
} -Body @{
    name = "Hamsa Vilas"
    owner_email = $OwnerEmail
    owner_password = $OwnerPassword
}
$ownerToken = $provisioned.tokens.access_token
$orgId = $provisioned.org_id

$branchResponse = Invoke-HamsaApi -Method Get -Path "/branches" -Token $ownerToken
$branches = @($branchResponse | Write-Output)
$mainBranch = $branches | Where-Object { $_.name -eq "Main Branch" } | Select-Object -First 1
Invoke-HamsaApi -Method Patch -Path "/branches/$($mainBranch.id)" -Token $ownerToken -Body @{
    name = "Koramangala"
    address = "18, 5th Cross, Koramangala, Bengaluru"
} | Out-Null
Invoke-HamsaApi -Method Post -Path "/branches" -Token $ownerToken -Body @{
    name = "Indiranagar"
} | Out-Null

$branchResponse = Invoke-HamsaApi -Method Get -Path "/branches" -Token $ownerToken
$branches = @($branchResponse | Write-Output)
$koramangala = $branches | Where-Object { $_.name -eq "Koramangala" } | Select-Object -First 1
$indiranagar = $branches | Where-Object { $_.name -eq "Indiranagar" } | Select-Object -First 1

$room101 = Invoke-HamsaApi -Method Post -Path "/branches/$($koramangala.id)/rooms" -Token $ownerToken -Body @{ number = "101" }
$room102 = Invoke-HamsaApi -Method Post -Path "/branches/$($koramangala.id)/rooms" -Token $ownerToken -Body @{ number = "102" }
$room201 = Invoke-HamsaApi -Method Post -Path "/branches/$($indiranagar.id)/rooms" -Token $ownerToken -Body @{ number = "201" }

$bed101A = Invoke-HamsaApi -Method Post -Path "/rooms/$($room101.id)/beds" -Token $ownerToken -Body @{ label = "A" }
$bed101B = Invoke-HamsaApi -Method Post -Path "/rooms/$($room101.id)/beds" -Token $ownerToken -Body @{ label = "B" }
$bed102A = Invoke-HamsaApi -Method Post -Path "/rooms/$($room102.id)/beds" -Token $ownerToken -Body @{ label = "A" }
$bed201A = Invoke-HamsaApi -Method Post -Path "/rooms/$($room201.id)/beds" -Token $ownerToken -Body @{ label = "A" }

$arjun = Invoke-HamsaApi -Method Post -Path "/branches/$($koramangala.id)/residents" -Token $ownerToken -Body @{
    full_name = "Arjun Rao"
    phone = "+919900001001"
    plan_kind = "monthly"
    rent_paise = 1500000
    deposit_paise = 3000000
}
$kavya = Invoke-HamsaApi -Method Post -Path "/branches/$($koramangala.id)/residents" -Token $ownerToken -Body @{
    full_name = "Kavya Iyer"
    phone = "+919900001002"
    plan_kind = "monthly"
    rent_paise = 1350000
    deposit_paise = 2700000
}
$rahul = Invoke-HamsaApi -Method Post -Path "/branches/$($koramangala.id)/residents" -Token $ownerToken -Body @{
    full_name = "Rahul Das"
    phone = "+919900001003"
    plan_kind = "weekly"
    rent_paise = 450000
    deposit_paise = 900000
}
$sana = Invoke-HamsaApi -Method Post -Path "/branches/$($indiranagar.id)/residents" -Token $ownerToken -Body @{
    full_name = "Sana Khan"
    phone = "+919900001004"
    plan_kind = "daily"
    rent_paise = 90000
    deposit_paise = 0
}

$managerInvite = Invoke-HamsaApi -Method Post -Path "/auth/invite" -Token $ownerToken -Body @{
    email = $ManagerEmail
    role = "manager"
    branch_id = $koramangala.id
    full_name = "Meera Sharma"
}
$managerTokens = Invoke-HamsaApi -Method Post -Path "/auth/invite/redeem" -Body @{
    invite_token = $managerInvite.invite_token
    password = $ManagerPassword
}
$managerToken = $managerTokens.access_token

$tenantInvite = Invoke-HamsaApi -Method Post -Path "/auth/invite" -Token $ownerToken -Body @{
    email = $TenantEmail
    role = "tenant"
    branch_id = $koramangala.id
    full_name = "Arjun Rao"
}
$tenantTokens = Invoke-HamsaApi -Method Post -Path "/auth/invite/redeem" -Body @{
    invite_token = $tenantInvite.invite_token
    password = $TenantPassword
}
$tenantToken = $tenantTokens.access_token

Invoke-HamsaApi -Method Post -Path "/auth/verify-identity" -Token $tenantToken -Body @{
    method = "mock"
    provider_ref = "local-dev-arjun"
    provider_name = "Arjun Rao"
    phone = "+919900001001"
    emergency_contact = "+919900009999"
    country = "IN"
} | Out-Null
Invoke-HamsaApi -Method Post -Path "/residents/$($arjun.id)/link-user" -Token $ownerToken -Body @{
    user_id = $tenantInvite.user_id
} | Out-Null

# The current API creates beds and residents separately but has no bed-assignment
# endpoint yet. Seed occupancy and realistic join dates directly in the dev DB.
$occupancySql = @"
BEGIN;
SET LOCAL search_path TO pool, public;
SET LOCAL app.current_org = '$orgId';
UPDATE residents SET bed_id = '$($bed101A.id)'::uuid, status = 'active', created_at = now() - interval '45 days' WHERE id = '$($arjun.id)'::uuid;
UPDATE residents SET bed_id = '$($bed101B.id)'::uuid, status = 'active', created_at = now() - interval '20 days' WHERE id = '$($kavya.id)'::uuid;
UPDATE residents SET bed_id = '$($bed102A.id)'::uuid, status = 'active', created_at = now() - interval '10 days' WHERE id = '$($rahul.id)'::uuid;
UPDATE residents SET bed_id = '$($bed201A.id)'::uuid, status = 'active', created_at = now() - interval '3 days' WHERE id = '$($sana.id)'::uuid;
COMMIT;
"@
docker exec hamsa-vilas-postgres psql -v ON_ERROR_STOP=1 -U hamsa -d hamsa_vilas -c $occupancySql | Out-Null

Invoke-HamsaApi -Method Put -Path "/org/receipt-template" -Token $ownerToken -Body @{
    display_name = "Hamsa Vilas"
    address = "Bengaluru, Karnataka"
    footer = "Thank you. Please retain this receipt."
    accent = "#1565C0"
    logo_key = $null
} | Out-Null

Invoke-HamsaApi -Method Post -Path "/residents/$($arjun.id)/payments" -Token $managerToken -Body @{ amount_paise = 1500000 } | Out-Null
Invoke-HamsaApi -Method Post -Path "/residents/$($kavya.id)/payments" -Token $managerToken -Body @{ amount_paise = 900000 } | Out-Null
Invoke-HamsaApi -Method Post -Path "/residents/$($rahul.id)/payments" -Token $managerToken -Body @{ amount_paise = 450000 } | Out-Null

Invoke-HamsaApi -Method Post -Path "/notices" -Token $ownerToken -Body @{
    title = "Rent collection reminder"
    body = "Monthly rent is due by the 5th. Contact your branch manager for a cash receipt."
    branch_id = $null
} | Out-Null
Invoke-HamsaApi -Method Post -Path "/notices" -Token $managerToken -Body @{
    title = "Water maintenance"
    body = "Water supply at Koramangala will pause from 11:00 AM to 1:00 PM tomorrow."
    branch_id = $koramangala.id
} | Out-Null

$tenantComplaint = Invoke-HamsaApi -Method Post -Path "/complaints" -Token $tenantToken -Body @{
    title = "Ceiling fan is noisy"
    description = "The fan in room 101 makes a rattling noise at medium speed."
    business_level = $false
    branch_id = $koramangala.id
}
Invoke-HamsaApi -Method Post -Path "/complaints/$($tenantComplaint.id)/transition" -Token $managerToken -Body @{ to = "assigned" } | Out-Null
Invoke-HamsaApi -Method Post -Path "/complaints/$($tenantComplaint.id)/transition" -Token $managerToken -Body @{ to = "in_progress" } | Out-Null
Invoke-HamsaApi -Method Post -Path "/complaints" -Token $ownerToken -Body @{
    title = "Annual fire-safety renewal"
    description = "Track the private compliance renewal for all branches."
    business_level = $true
    branch_id = $null
} | Out-Null

Invoke-HamsaApi -Method Put -Path "/listings/$($koramangala.id)" -Token $ownerToken -Body @{
    title = "Hamsa Vilas Koramangala"
    area = "Koramangala"
    city = "Bengaluru"
    lat = 12.9352
    lng = 77.6245
    description = "Well-managed shared accommodation near the startup district."
    amenities = "Wi-Fi, power backup, housekeeping, hot water"
    categories = @(
        @{ name = "Double sharing"; price_paise = 1500000; seats = 2 },
        @{ name = "Triple sharing"; price_paise = 1250000; seats = 3 }
    )
    published = $true
} | Out-Null
Invoke-HamsaApi -Method Put -Path "/listings/$($indiranagar.id)" -Token $ownerToken -Body @{
    title = "Hamsa Vilas Indiranagar"
    area = "Indiranagar"
    city = "Bengaluru"
    lat = 12.9784
    lng = 77.6408
    description = "Comfortable accommodation close to the Metro."
    amenities = "Wi-Fi, laundry, housekeeping"
    categories = @(
        @{ name = "Daily stay"; price_paise = 90000; seats = 1 }
    )
    published = $true
} | Out-Null

# Leave one manager edit pending so the owner's maker-checker screen has data.
Invoke-HamsaApi -Method Patch -Path "/rooms/$($room102.id)" -Token $managerToken -Body @{
    number = "102-A"
} | Out-Null

$tenantLedger = Invoke-HamsaApi -Method Get -Path "/me/ledger" -Token $tenantToken
$tenantReceiptResponse = Invoke-HamsaApi -Method Get -Path "/me/receipts" -Token $tenantToken
$tenantReceipts = @($tenantReceiptResponse | Write-Output)
$finance = Invoke-HamsaApi -Method Get -Path "/finance/org" -Token $ownerToken
$pendingChangeResponse = Invoke-HamsaApi -Method Get -Path "/change-requests" -Token $ownerToken
$pendingChanges = @($pendingChangeResponse | Write-Output)

Write-Host "Development data created successfully."
Write-Host "Owner:   $OwnerEmail / $OwnerPassword"
Write-Host "Manager: $ManagerEmail / $ManagerPassword"
Write-Host "Tenant:  $TenantEmail / $TenantPassword"
Write-Host "Tenant outstanding (paise): $($tenantLedger.outstanding_paise)"
Write-Host "Tenant receipts: $($tenantReceipts.Count)"
Write-Host "Branches in finance report: $($finance.branches.Count)"
Write-Host "Pending approvals: $($pendingChanges.Count)"
