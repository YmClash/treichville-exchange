mod service;
mod jwt;
pub mod middleware;
mod dto;
mod repository;

pub use service::{AuthService, LoginRequest, LoginResponse, RegisterRequest, TokenPair};
pub use jwt::{JwtClaims, JwtService};
pub use middleware::{AuthMiddleware, CurrentUser, require_auth};
pub use dto::{RegisterDto, LoginDto, RefreshTokenDto, ChangePasswordDto, UpdateProfileDto, Enable2FADto, Verify2FADto};
pub use repository::{UserRepository, UserRepositoryTrait};