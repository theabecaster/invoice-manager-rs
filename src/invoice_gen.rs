use anyhow::Result;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::process::Command;

use crate::models::{Invoice, InvoiceLineItem, Profile, Client, Project};

/// Service for generating invoice files in Markdown and PDF format
pub struct InvoiceGenerator {
    output_dir: String,
}

impl InvoiceGenerator {
    pub fn new(output_dir: &str) -> Result<Self> {
        // Create the output directory if it doesn't exist
        let path = Path::new(output_dir);
        if !path.exists() {
            fs::create_dir_all(path)?;
        }
        
        Ok(Self {
            output_dir: output_dir.to_string(),
        })
    }
    
    /// Generate a Markdown invoice file and convert it to PDF using pandoc if available
    pub fn generate_invoice(
        &self, 
        invoice: &Invoice, 
        line_items: &[InvoiceLineItem],
        profile: &Profile,
        client: &Client,
        project: &Project
    ) -> Result<(String, String)> {
        // Generate Markdown content
        let markdown = self.generate_markdown(invoice, line_items, profile, client, project)?;
        
        // Create file names
        let md_filename = format!("invoice_{}.md", invoice.number);
        let pdf_filename = format!("invoice_{}.pdf", invoice.number);
        
        // Construct full paths
        let md_path = format!("{}/{}", self.output_dir, md_filename);
        let pdf_path = format!("{}/{}", self.output_dir, pdf_filename);
        
        // Write Markdown to file
        let mut file = File::create(&md_path)?;
        file.write_all(markdown.as_bytes())?;
        
        // Try to generate PDF using pandoc
        let pdf_result = Command::new("pandoc")
            .arg(&md_path)
            .arg("-o")
            .arg(&pdf_path)
            .output();
        
        match pdf_result {
            Ok(output) => {
                if !output.status.success() {
                    let error = String::from_utf8_lossy(&output.stderr);
                    println!("Warning: Failed to generate PDF: {}", error);
                    // Create a simple text file as PDF substitute
                    self.create_markdown_copy(&md_path, &pdf_path)?;
                }
            }
            Err(e) => {
                println!("Warning: Could not run pandoc: {}", e);
                // Create a simple text file as PDF substitute
                self.create_markdown_copy(&md_path, &pdf_path)?;
            }
        }
        
        Ok((md_path, pdf_path))
    }
    
    /// Create a copy of the markdown file with .pdf extension as fallback
    fn create_markdown_copy(&self, md_path: &str, pdf_path: &str) -> Result<()> {
        // Read markdown content
        let content = fs::read_to_string(md_path)?;
        
        // Write the same content to the PDF path (it's not a real PDF but will be available for attachment)
        let mut file = File::create(pdf_path)?;
        file.write_all(content.as_bytes())?;
        
        println!("Created markdown copy as PDF substitute: {}", pdf_path);
        Ok(())
    }
    
    /// Generate Markdown content for the invoice
    fn generate_markdown(
        &self, 
        invoice: &Invoice, 
        line_items: &[InvoiceLineItem],
        profile: &Profile,
        client: &Client,
        project: &Project
    ) -> Result<String> {
        let mut content = String::new();
        
        // Add top blue divider
        content.push_str("<hr style=\"height: 5px; background-color: #343876; border: none;\">\n\n");
        
        // Add profile header (name, address, phone)
        content.push_str(&format!("# {}\n", profile.name));
        
        // Address is optional, handle appropriately
        if let Some(address) = &profile.address {
            content.push_str(&format!("{}\n", address));
        }
        
        content.push_str(&format!("{}\n\n", profile.phonenumber));
        
        // Add Invoice title
        content.push_str("# Invoice\n");
        content.push_str(&format!("<span style=\"color: #e83e8c;\">Submitted on {}</span>\n\n", invoice.submit_date.format("%m/%d/%Y")));
        
        // Create two column layout for client and payment info
        content.push_str("<div style=\"display: flex; justify-content: space-between;\">\n");
        
        // Left column - Invoice for
        content.push_str("<div style=\"width: 30%;\">\n");
        content.push_str("**Invoice for**<br>\n");
        content.push_str(&format!("{}\n", client.name));
        content.push_str("</div>\n");
        
        // Middle column - Payable to
        content.push_str("<div style=\"width: 40%;\">\n");
        content.push_str("**Payable to**<br>\n");
        content.push_str(&format!("{}<br>\n", profile.name));
        content.push_str("<br>\n");
        content.push_str("**Account Number**<br>\n");
        content.push_str(&format!("{}<br>\n", profile.bank_account_number));
        content.push_str("<br>\n");
        content.push_str("**Routing Number**<br>\n");
        content.push_str(&format!("{}\n", profile.bank_routing_number));
        content.push_str("</div>\n");
        
        // Right column - Invoice number
        content.push_str("<div style=\"width: 30%;\">\n");
        content.push_str("**Invoice #**<br>\n");
        content.push_str(&format!("{}\n", invoice.number));
        content.push_str("</div>\n");
        
        content.push_str("</div>\n\n");
        
        // Add horizontal divider
        content.push_str("<hr>\n\n");
        
        // Add line items table with better formatting
        content.push_str("<table style=\"width: 100%; border-collapse: collapse;\">\n");
        
        // Table header
        content.push_str("<tr>\n");
        content.push_str("<th style=\"text-align: left;\">Description</th>\n");
        content.push_str("<th style=\"text-align: right;\">Hours</th>\n");
        content.push_str("<th style=\"text-align: right;\">Hourly rate</th>\n");
        content.push_str("<th style=\"text-align: right;\">Total price</th>\n");
        content.push_str("</tr>\n");
        
        let mut total_hours = 0.0;
        let mut total_amount = 0.0;
        
        // Table rows for each line item
        for item in line_items {
            let amount = item.hours * invoice.rate;
            total_hours += item.hours;
            total_amount += amount;
            
            content.push_str("<tr>\n");
            content.push_str(&format!("<td style=\"text-align: left;\">{}</td>\n", item.description));
            content.push_str(&format!("<td style=\"text-align: right;\">{}</td>\n", item.hours));
            content.push_str(&format!("<td style=\"text-align: right;\">${:.2}</td>\n", invoice.rate));
            content.push_str(&format!("<td style=\"text-align: right;\">${:.2}</td>\n", amount));
            content.push_str("</tr>\n");
        }
        
        // Add total row
        content.push_str("<tr>\n");
        content.push_str("<td colspan=\"3\" style=\"text-align: right;\">Total</td>\n");
        content.push_str(&format!("<td style=\"text-align: right; font-weight: bold; color: #e83e8c;\"><h2>${:.2}</h2></td>\n", total_amount));
        content.push_str("</tr>\n");
        
        content.push_str("</table>\n");
        
        Ok(content)
    }
} 